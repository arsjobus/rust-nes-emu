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
        .expect("Could not create window");

    // New minifb API.
    window.set_target_fps(60);

    /*
     * Start the audio output device.
     *
     * This must stay alive for the entire emulator session.
     * The Audio object owns the CPAL stream.
     */
    let mut audio =
        Audio::new()
            .expect("Could not initialize audio");

    /*
     * Post-processing pipeline.
     *
     * Effects are applied in the order they are added.
     *
     * PPU framebuffer
     *      ↓
     * NTSC color bleed
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
     *
     * strength:
     *     How strongly color information bleeds
     *     horizontally.
     *
     * bleed:
     *     Number of pixels used for the horizontal
     *     chroma blur.
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
     * Bloom
     *
     * threshold:
     *     Minimum brightness required to contribute
     *     to the glow.
     *
     * strength:
     *     Amount of glow added to the image.
     *
     * radius:
     *     Size of the glow around bright pixels.
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
     *     0.0 = unchanged.
     *
     * contrast:
     *     1.0 = unchanged.
     *
     * saturation:
     *     1.0 = unchanged.
     *
     * gamma:
     *     1.0 = unchanged.
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
     * LUT
     *
     * Built-in LUT-style color transform.
     *
     * Available presets include:
     *
     * Identity
     * WarmCrt
     * CoolCrt
     * Composite
     * GameBoy
     * Amber
     * HighContrast
     *
     * strength:
     *     0.0 = original colors
     *     1.0 = full LUT effect
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
     * Scanlines
     *
     * Darkens alternating horizontal lines.
     */
    postprocess.add(
        Scanlines::new(
            0.1
        )
    );

    postprocess.set_enabled(
        "scanlines",
        false,
    );

    /*
     * Vignette
     *
     * Slightly darkens the edges of the screen.
     */
    postprocess.add(
        Vignette::new(
            0.35
        )
    );

    postprocess.set_enabled(
        "vignette",
        false,
    );

    let max_frames =
        env::var("NES_MAX_FRAMES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok());

    let mut frame_count =
        0u64;

    let mut last =
        Instant::now();

    while window.is_open()
        && !window.is_key_down(Key::Escape)
    {
        /*
         * Update controller state before running
         * the next frame.
         */
        nes.update_input(
            &window
        );

        /*
         * Run the CPU/PPU/APU until one video frame
         * has completed.
         */
        nes.run_frame();

        /*
         * The APU generated PCM samples while the
         * frame was executing.
         *
         * Send those samples to the real audio device.
         */
        let samples =
            nes.take_audio_samples();

        audio.push_samples(
            &samples
        );

        /*
         * Apply post-processing after the PPU has
         * completely rendered the current frame.
         *
         * The framebuffer contains 256x240 RGB pixels
         * stored as u32 values in 0xRRGGBB format.
         */
        postprocess.apply(
            nes.framebuffer_mut(),
            WIDTH,
            HEIGHT,
        );

        /*
         * Draw the completed, post-processed video frame.
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
         * Keep the emulator around 60 FPS.
         *
         * The audio stream itself runs independently,
         * so this sleep does not directly play the audio.
         */
        let elapsed =
            last.elapsed();

        if elapsed <
            Duration::from_micros(
                16_667
            )
        {
            std::thread::sleep(
                Duration::from_micros(
                    16_667
                ) - elapsed
            );
        }

        last =
            Instant::now();
    }
}
