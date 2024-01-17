use avif_serialize::constants::MatrixCoefficients as AvifMatrixCoefficients;
use avif_serialize::Aviffy;
use core::iter::zip;
use imgref::{Img, ImgRef};
use rav1e::color::{
    ChromaSamplePosition, ChromaSampling, ColorDescription, ColorPrimaries, MatrixCoefficients,
    PixelRange, TransferCharacteristics,
};
use rav1e::config::{Config, EncoderConfig, PredictionModesSetting, SpeedSettings};
use rav1e::data::{FrameType, Rational};
use rav1e::prelude::{
    BlockSize, MotionSpeedSettings, PartitionRange, PartitionSpeedSettings,
    PredictionSpeedSettings, SGRComplexityLevel, SceneDetectionSpeed, SegmentationLevel,
    TransformSpeedSettings, Tune,
};
use rav1e::EncoderStatus;
use rgb::{FromSlice, RGBA8};

const WIDTH: usize = 36;
const HEIGHT: usize = 36;

const BIT_DEPTH: usize = 8;

fn main() {
    let data = include_bytes!("./test.bin");

    let buffer = Img::new(data.as_rgba(), WIDTH, HEIGHT);

    let rgb_data = encode_data(buffer, PixelKind::Rgb);
    let alpha_data = encode_data(buffer, PixelKind::Alpha);

    drop(
        Aviffy::new()
            .matrix_coefficients(AvifMatrixCoefficients::Rgb)
            .to_vec(
                &rgb_data,
                Some(&alpha_data),
                u32::try_from(WIDTH).unwrap(),
                u32::try_from(HEIGHT).unwrap(),
                u8::try_from(BIT_DEPTH).unwrap(),
            ),
    );
}

#[derive(Clone, Copy)]
enum PixelKind {
    Rgb,
    Alpha,
}

fn encode_data(buffer: ImgRef<'_, RGBA8>, kind: PixelKind) -> Vec<u8> {
    let encoder_config = get_encoder_config(kind);
    let mut context = Config::new()
        .with_encoder_config(encoder_config)
        .new_context()
        .unwrap();
    let mut frame = context.new_frame();

    // can this iterator weirdness be avoided by directly constructing a Plane?

    match kind {
        PixelKind::Rgb => {
            let [mut plane_y, mut plane_u, mut plane_v] = frame.planes;

            plane_y
                .rows_iter_mut()
                .zip(plane_u.rows_iter_mut())
                .zip(plane_v.rows_iter_mut())
                .take(HEIGHT)
                .flat_map(|((y, u), v)| zip(zip(y, u), v).take(WIDTH))
                .zip(buffer.pixels())
                .for_each(|(((y, u), v), pixel)| {
                    *y = pixel.g;
                    *u = pixel.b;
                    *v = pixel.r;
                });

            frame.planes = [plane_y, plane_u, plane_v];
        }
        PixelKind::Alpha => {
            frame.planes[0]
                .rows_iter_mut()
                .take(HEIGHT)
                .flat_map(|a| a.iter_mut().take(WIDTH))
                .zip(buffer.pixels())
                .for_each(|(a, pixel)| {
                    *a = pixel.a;
                });
        }
    }

    context.send_frame(frame).unwrap();
    context.flush();

    let mut encoded_data = Vec::new();

    loop {
        match context.receive_packet() {
            Ok(mut packet) => match packet.frame_type {
                FrameType::KEY => {
                    encoded_data.append(&mut packet.data);
                }
                _ => continue,
            },
            Err(EncoderStatus::Encoded | EncoderStatus::LimitReached) => break,
            Err(err) => panic!("{err}"),
        }
    }

    encoded_data
}

fn get_encoder_config(kind: PixelKind) -> EncoderConfig {
    const QUANTIZER: usize = 121; // default quality in `ravif` is 80, which becomes 121

    const THREADS: usize = 1; // the tile count depends on this?

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
        tiles: THREADS,
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
