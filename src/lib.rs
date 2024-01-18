use imgref::ImgRef;
use rav1e::prelude::*;
use rgb::{RGB8, RGBA8};

pub fn encode_rgba(buffer: ImgRef<RGBA8>) {
    let use_alpha = buffer.pixels().any(|px| px.a != 255);

    if use_alpha {
        let planes = buffer.pixels().map(|px| [px.g, px.b, px.r]);
        encode_raw_planes(planes)
    } else {
        let planes = buffer.pixels().map(|px| {
            let (y, u, v) = rgb_to_10_bit_gbr(px.rgb());
            [y, u, v]
        });
        encode_raw_planes(planes)
    }
}

fn encode_raw_planes<P: Pixel>(planes: impl IntoIterator<Item = [P; 3]> + Send) {
    rayon::join(
        || {},
        || {
            let mut ctx = Config::new()
                .with_encoder_config(get_encoder_config())
                .new_context::<P>()
                .unwrap();
            let frame = ctx.new_frame();

            ctx.send_frame(frame).unwrap();
            ctx.flush();

            drop(ctx.receive_packet().unwrap());
        },
    );
}

fn to_ten(x: u8) -> u16 {
    (u16::from(x) << 2) | (u16::from(x) >> 6)
}

fn rgb_to_10_bit_gbr(px: RGB8) -> (u16, u16, u16) {
    (to_ten(px.g), to_ten(px.b), to_ten(px.r))
}

fn get_encoder_config() -> EncoderConfig {
    const WIDTH: usize = 180;
    const HEIGHT: usize = 180;

    const BIT_DEPTH: usize = 8;

    const QUANTIZER: usize = 121; // default quality in `ravif` is 80, which becomes 121

    let tiles = {
        let threads = rayon::current_num_threads();
        threads.min((WIDTH * HEIGHT) / 128usize.pow(2))
    };

    let speed_settings = get_speed_settings();

    EncoderConfig {
        width: WIDTH,
        height: HEIGHT,
        sample_aspect_ratio: Rational::new(1, 1),
        time_base: Rational::new(1, 1),
        bit_depth: BIT_DEPTH,
        chroma_sampling: ChromaSampling::Cs400,
        chroma_sample_position: ChromaSamplePosition::Unknown,
        pixel_range: PixelRange::Full,
        color_description: None,
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
