use crate::error::Error;
use imgref::Img;
use rav1e::prelude::*;

pub fn encode_rgba(buffer: Img<&[rgb::RGBA<u8>]>) -> Result<(), Error> {
    let width = buffer.width();
    let height = buffer.height();

    let use_alpha = buffer.pixels().any(|px| px.a != 255);

    if use_alpha {
        let planes = buffer.pixels().map(|px| {
            let (y, u, v) = rgb_to_8_bit_gbr(px.rgb());
            [y, u, v]
        });
        let alpha = buffer.pixels().map(|px| px.a);
        encode_raw_planes(width, height, planes, Some(alpha))
    } else {
        let planes = buffer.pixels().map(|px| {
            let (y, u, v) = rgb_to_10_bit_gbr(px.rgb());
            [y, u, v]
        });
        encode_raw_planes(width, height, planes, None::<[_; 0]>)
    }
}

#[inline(never)]
fn encode_raw_planes<P: rav1e::Pixel + Default>(
    width: usize,
    height: usize,
    planes: impl IntoIterator<Item = [P; 3]> + Send,
    alpha: Option<impl IntoIterator<Item = P> + Send>,
) -> Result<(), Error> {
    let encode_color = move || {
        encode_to_av1::<P>(PixelKind::Rgb, move |frame| {
            init_frame_3(width, height, planes, frame)
        })
    };
    let encode_alpha = move || {
        alpha.map(|alpha| {
            encode_to_av1::<P>(PixelKind::Alpha, |frame| {
                init_frame_1(width, height, alpha, frame)
            })
        })
    };
    let (color, alpha) = rayon::join(encode_color, encode_alpha);
    color?;
    alpha.transpose()?;

    Ok(())
}

#[inline(always)]
fn to_ten(x: u8) -> u16 {
    (u16::from(x) << 2) | (u16::from(x) >> 6)
}

#[inline(always)]
fn rgb_to_10_bit_gbr(px: rgb::RGB<u8>) -> (u16, u16, u16) {
    (to_ten(px.g), to_ten(px.b), to_ten(px.r))
}

#[inline(always)]
fn rgb_to_8_bit_gbr(px: rgb::RGB<u8>) -> (u8, u8, u8) {
    (px.g, px.b, px.r)
}

fn init_frame_3<P: rav1e::Pixel + Default>(
    width: usize,
    height: usize,
    planes: impl IntoIterator<Item = [P; 3]> + Send,
    frame: &mut Frame<P>,
) -> Result<(), Error> {
    let mut f = frame.planes.iter_mut();
    let mut planes = planes.into_iter();

    // it doesn't seem to be necessary to fill padding area
    let mut y = f.next().unwrap().mut_slice(Default::default());
    let mut u = f.next().unwrap().mut_slice(Default::default());
    let mut v = f.next().unwrap().mut_slice(Default::default());

    for ((y, u), v) in y
        .rows_iter_mut()
        .zip(u.rows_iter_mut())
        .zip(v.rows_iter_mut())
        .take(height)
    {
        let y = &mut y[..width];
        let u = &mut u[..width];
        let v = &mut v[..width];
        for ((y, u), v) in y.iter_mut().zip(u).zip(v) {
            let px = planes.next().ok_or(Error::TooFewPixels)?;
            *y = px[0];
            *u = px[1];
            *v = px[2];
        }
    }
    Ok(())
}

fn init_frame_1<P: rav1e::Pixel + Default>(
    width: usize,
    height: usize,
    planes: impl IntoIterator<Item = P> + Send,
    frame: &mut Frame<P>,
) -> Result<(), Error> {
    let mut y = frame.planes[0].mut_slice(Default::default());
    let mut planes = planes.into_iter();

    for y in y.rows_iter_mut().take(height) {
        let y = &mut y[..width];
        for y in y.iter_mut() {
            *y = planes.next().ok_or(Error::TooFewPixels)?;
        }
    }
    Ok(())
}

#[inline(never)]
fn encode_to_av1<P: rav1e::Pixel>(
    kind: PixelKind,
    init: impl FnOnce(&mut Frame<P>) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut ctx: Context<P> = Config::new()
        .with_encoder_config(get_encoder_config(kind))
        .new_context()?;
    let frame = ctx.new_frame();

    ctx.send_frame(frame)?;
    ctx.flush();

    drop(ctx.receive_packet()?);

    Ok(())
}

enum PixelKind {
    Rgb,
    Alpha,
}

fn get_encoder_config(kind: PixelKind) -> EncoderConfig {
    const WIDTH: usize = 180;
    const HEIGHT: usize = 180;

    const BIT_DEPTH: usize = 8;

    const QUANTIZER: usize = 121; // default quality in `ravif` is 80, which becomes 121

    let tiles = {
        let threads = rayon::current_num_threads();
        threads.min((WIDTH * HEIGHT) / 128usize.pow(2))
    };

    let chroma_sampling = match kind {
        PixelKind::Rgb => ChromaSampling::Cs444,
        PixelKind::Alpha => ChromaSampling::Cs400,
    };

    let color_description = match kind {
        PixelKind::Rgb => Some(ColorDescription {
            color_primaries: ColorPrimaries::BT709,
            transfer_characteristics: TransferCharacteristics::SRGB,
            matrix_coefficients: MatrixCoefficients::Identity,
        }),
        PixelKind::Alpha => None,
    };

    let speed_settings = get_speed_settings();

    EncoderConfig {
        width: WIDTH,
        height: HEIGHT,
        sample_aspect_ratio: Rational::new(1, 1),
        time_base: Rational::new(1, 1),
        bit_depth: BIT_DEPTH,
        chroma_sampling,
        chroma_sample_position: ChromaSamplePosition::Unknown,
        pixel_range: PixelRange::Full,
        color_description,
        mastering_display: None,
        content_light: None,
        enable_timing_info: false,
        level_idx: None,
        still_picture: true,
        error_resilient: false,
        switch_frame_interval: 0,
        min_key_frame_interval: 0,
        max_key_frame_interval: 0,
        reservoir_frame_delay: None,
        low_latency: false,
        quantizer: QUANTIZER,
        min_quantizer: u8::try_from(QUANTIZER).unwrap(),
        bitrate: 0,
        tune: Tune::Psychovisual,
        film_grain_params: None,
        tile_cols: 0,
        tile_rows: 0,
        tiles,
        speed_settings,
    }
}

fn get_speed_settings() -> SpeedSettings {
    let mut settings = SpeedSettings::default();

    // These are all of the speed settings.
    // SpeedSettings cannot be created using struct literal syntax because it is marked as `non_exhaustive`.
    settings.multiref = false;
    settings.fast_deblock = false;
    settings.rdo_lookahead_frames = 1;
    settings.scene_detection_mode = SceneDetectionSpeed::None;
    settings.cdef = false;
    settings.lrf = false;
    settings.lru_on_skip = false;
    settings.sgr_complexity = SGRComplexityLevel::Reduced;
    settings.segmentation = SegmentationLevel::Simple;
    settings.partition = PartitionSpeedSettings {
        encode_bottomup: false,
        non_square_partition_max_threshold: BlockSize::BLOCK_8X8,
        partition_range: PartitionRange::new(BlockSize::BLOCK_8X8, BlockSize::BLOCK_16X16),
    };
    settings.transform = TransformSpeedSettings {
        reduced_tx_set: false,
        tx_domain_distortion: true,
        tx_domain_rate: false,
        rdo_tx_decision: false,
        enable_inter_tx_split: false,
    };
    settings.prediction = PredictionSpeedSettings {
        prediction_modes: PredictionModesSetting::Simple,
        fine_directional_intra: true,
    };
    settings.motion = MotionSpeedSettings {
        use_satd_subpel: false,
        include_near_mvs: false,
        me_allow_full_search: true,
    };

    settings
}
