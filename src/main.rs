use rav1e::color::{
    ChromaSamplePosition, ChromaSampling, ColorDescription, ColorPrimaries, MatrixCoefficients,
    PixelRange, TransferCharacteristics,
};
use rav1e::config::{Config, EncoderConfig, PredictionModesSetting, SpeedSettings};
use rav1e::data::Rational;
use rav1e::prelude::{
    BlockSize, MotionSpeedSettings, PartitionRange, PartitionSpeedSettings,
    PredictionSpeedSettings, SGRComplexityLevel, SceneDetectionSpeed, SegmentationLevel,
    TransformSpeedSettings, Tune,
};

fn main() {
    let mut context = Config::new()
        .with_encoder_config(get_encoder_config())
        .new_context::<u8>()
        .unwrap();

    let frame = context.new_frame();
    context.send_frame(frame).unwrap();
    context.flush();

    drop(context.receive_packet().unwrap());
}

fn get_encoder_config() -> EncoderConfig {
    let speed_settings = get_speed_settings();

    EncoderConfig {
        width: 36,
        height: 36,
        sample_aspect_ratio: Rational::new(1, 1),
        time_base: Rational::new(1, 1),
        bit_depth: 8,
        chroma_sampling: ChromaSampling::Cs444,
        chroma_sample_position: ChromaSamplePosition::Unknown,
        pixel_range: PixelRange::Full,
        color_description: Some(ColorDescription {
            color_primaries: ColorPrimaries::BT709,
            transfer_characteristics: TransferCharacteristics::SRGB,
            matrix_coefficients: MatrixCoefficients::Identity,
        }),
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
        quantizer: 121, // default quality in `ravif` is 80, which becomes 121
        min_quantizer: 121,
        bitrate: 0,
        tune: Tune::Psychovisual,
        film_grain_params: None,
        tile_cols: 0,
        tile_rows: 0,
        tiles: 1,
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
