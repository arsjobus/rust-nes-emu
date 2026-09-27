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
    Bloom,
    ColorCorrection,
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
     * The second argument controls how many
     * previous frames are retained.
     *
     * 0.10, 3 = subtle
     * 0.20, 3 = noticeable
     * 0.35, 3 = strong
     * 0.50, 4 = very strong
     *
     * START WITH 0.35 so the effect is obvious.
     */
    postprocess.add(
        Persistence::new(
            0.2,
            3,
        )
    );

    /*
     * ENABLED FOR TESTING.
     *
     * Once you've confirmed it works, you can
     * change this back to false.
     */
    postprocess.set_enabled(
        "persistence",
        false,
    );

    /*
     * Bloom.
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
