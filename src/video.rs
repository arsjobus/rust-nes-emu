use minifb::{
    Key,
    Window,
    WindowOptions,
};

use std::{
    env,
    time::{
        Duration,
        Instant,
    },
};

use crate::audio::Audio;
use crate::nes::Nes;

use crate::postprocess::{
    AutoGradient,
    Bloom,
    ColorCorrection,
    Curvature,
    Lut,
    LutPreset,
    Ntsc,
    Persistence,
    PostProcessPipeline,
    Scanlines,
    Vignette,
};

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

pub fn run(
    mut nes: Nes,
) {
    let mut window =
        Window::new(
            "NES",
            WIDTH * 3,
            HEIGHT * 3,
            WindowOptions {
                resize: false,
                scale: minifb::Scale::X1,
                ..WindowOptions::default()
            },
        )
        .expect(
            "Could not create window"
        );

    window.set_target_fps(60);

    /*
     * Start audio.
     *
     * The Audio object owns the CPAL stream and
     * must remain alive for the emulator session.
     */
    let mut audio =
        Audio::new()
            .expect(
                "Could not initialize audio"
            );

    /*
     * Post-processing pipeline.
     *
     * PPU framebuffer
     *      ↓
     * NTSC
     *      ↓
     * Persistence
     *      ↓
     * Bloom
     *      ↓
     * Color correction
     *      ↓
     * LUT
     *      ↓
     * Curvature
     *      ↓
     * Auto Gradient
     *      ↓
     * Scanlines
     *      ↓
     * Vignette
     *      ↓
     * minifb
     */
    let mut postprocess =
        PostProcessPipeline::new();

    /*
     * NTSC / composite color bleed.
     *
     * strength:
     *     How strongly color information bleeds
     *     horizontally.
     *
     * bleed:
     *     Horizontal bleed distance.
     */
    postprocess.add(
        Ntsc::new(
            0.65,
            2,
        )
    );

    postprocess.set_enabled(
        "ntsc",
        false,
    );

    /*
     * Motion persistence / multi-frame blur.
     *
     * amount:
     *     Previous-frame contribution.
     *
     * frames:
     *     Number of previous frames retained.
     */
    postprocess.add(
        Persistence::new(
            0.2,
            3,
        )
    );

    postprocess.set_enabled(
        "persistence",
        false,
    );

    /*
     * Bloom.
     *
     * threshold:
     *     Minimum brightness required to glow.
     *
     * strength:
     *     Amount of glow.
     *
     * radius:
     *     Glow radius.
     */
    postprocess.add(
        Bloom::new(
            180,
            0.20,
            3,
        )
    );

    postprocess.set_enabled(
        "bloom",
        false,
    );

    /*
     * Color correction.
     *
     * brightness:
     *     0.0 = unchanged
     *
     * contrast:
     *     1.0 = unchanged
     *
     * saturation:
     *     1.0 = unchanged
     *
     * gamma:
     *     1.0 = unchanged
     */
    postprocess.add(
        ColorCorrection::new(
            0.0,
            1.05,
            0.95,
            1.0,
        )
    );

    postprocess.set_enabled(
        "color_correction",
        false,
    );

    /*
     * LUT.
     *
     * WarmCrt is configured but disabled.
     */
    postprocess.add(
        Lut::new(
            LutPreset::WarmCrt,
            0.65,
        )
    );

    postprocess.set_enabled(
        "lut",
        false,
    );

    /*
     * CRT curvature.
     *
     * 0.00 = flat
     * 0.05 = subtle
     * 0.10 = noticeable
     * 0.15 = strong
     * 0.25 = heavy
     */
    postprocess.add(
        Curvature::new(
            0.05,
        )
    );

    postprocess.set_enabled(
        "curvature",
        false,
    );

    /*
     * Automatic screen gradient.
     *
     * strength:
     *     Overall gradient intensity.
     *
     * vertical:
     *     Top-to-bottom illumination variation.
     *
     * horizontal:
     *     Center-to-edge illumination variation.
     *
     * This is enabled for testing.
     */
    postprocess.add(
        AutoGradient::new(
            0.15,
            0.40,
            0.25,
        )
    );

    postprocess.set_enabled(
        "auto_gradient",
        false,
    );

    /*
     * Scanlines.
     */
    postprocess.add(
        Scanlines::new(
            0.1,
        )
    );

    postprocess.set_enabled(
        "scanlines",
        false,
    );

    /*
     * Vignette.
     */
    postprocess.add(
        Vignette::new(
            0.35,
        )
    );

    postprocess.set_enabled(
        "vignette",
        false,
    );

    let max_frames =
        env::var("NES_MAX_FRAMES")
            .ok()
            .and_then(
                |v| v.parse::<u64>().ok()
            );

    let mut frame_count =
        0u64;

    let mut last =
        Instant::now();

    while window.is_open()
        && !window.is_key_down(
            Key::Escape
        )
    {
        /*
         * Update controller state.
         */
        nes.update_input(
            &window
        );

        /*
         * Run CPU/PPU/APU until one
         * complete video frame exists.
         */
        nes.run_frame();

        /*
         * Send generated audio samples
         * to the audio device.
         */
        let samples =
            nes.take_audio_samples();

        audio.push_samples(
            &samples
        );

        /*
         * Apply the post-processing pipeline.
         *
         * framebuffer:
         *     256x240 pixels
         *
         * format:
         *     0xRRGGBB
         */
        postprocess.apply(
            nes.framebuffer_mut(),
            WIDTH,
            HEIGHT,
        );

        /*
         * Display the processed frame.
         */
        window
            .update_with_buffer(
                nes.framebuffer(),
                WIDTH,
                HEIGHT,
            )
            .expect(
                "Failed to update window"
            );

        frame_count += 1;

        if let Some(max) =
            max_frames
        {
            if frame_count >= max {
                break;
            }
        }

        /*
         * Keep emulator around 60 FPS.
         */
        let elapsed =
            last.elapsed();

        if elapsed <
            Duration::from_micros(
                16_667,
            )
        {
            std::thread::sleep(
                Duration::from_micros(
                    16_667,
                ) - elapsed
            );
        }

        last =
            Instant::now();
    }
}
