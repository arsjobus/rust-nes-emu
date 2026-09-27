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
    args: &[String],
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

    let mut audio =
        Audio::new()
            .expect(
                "Could not initialize audio"
            );

    let mut postprocess =
        PostProcessPipeline::new();

    /*
     * ---------------------------------------------------------
     * NTSC
     * ---------------------------------------------------------
     */
    postprocess.add(
        Ntsc::new(
            0.65,
            2,
        )
    );

    postprocess.set_enabled(
        "ntsc",
        args.iter().any(
            |arg| arg == "--ntsc"
        ),
    );

    /*
     * ---------------------------------------------------------
     * Persistence
     * ---------------------------------------------------------
     */
    postprocess.add(
        Persistence::new(
            0.2,
            3,
        )
    );

    postprocess.set_enabled(
        "persistence",
        args.iter().any(
            |arg| arg == "--persistence"
        ),
    );

    /*
     * ---------------------------------------------------------
     * Bloom
     * ---------------------------------------------------------
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
        args.iter().any(
            |arg| arg == "--bloom"
        ),
    );

    /*
     * ---------------------------------------------------------
     * Color correction
     * ---------------------------------------------------------
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
        args.iter().any(
            |arg| arg == "--color-correction"
        ),
    );

    /*
     * ---------------------------------------------------------
     * LUT
     * ---------------------------------------------------------
     */
    postprocess.add(
        Lut::new(
            LutPreset::WarmCrt,
            0.65,
        )
    );

    postprocess.set_enabled(
        "lut",
        args.iter().any(
            |arg| arg == "--lut"
        ),
    );

    /*
     * ---------------------------------------------------------
     * CRT curvature
     * ---------------------------------------------------------
     */
    postprocess.add(
        Curvature::new(
            0.05,
        )
    );

    postprocess.set_enabled(
        "curvature",
        args.iter().any(
            |arg| arg == "--curvature"
        ),
    );

    /*
     * ---------------------------------------------------------
     * Automatic gradient
     * ---------------------------------------------------------
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
        args.iter().any(
            |arg| arg == "--auto-gradient"
        ),
    );

    /*
     * ---------------------------------------------------------
     * Scanlines
     * ---------------------------------------------------------
     */
    postprocess.add(
        Scanlines::new(
            0.1,
        )
    );

    postprocess.set_enabled(
        "scanlines",
        args.iter().any(
            |arg| arg == "--scanlines"
        ),
    );

    /*
     * ---------------------------------------------------------
     * Vignette
     * ---------------------------------------------------------
     */
    postprocess.add(
        Vignette::new(
            0.35,
        )
    );

    postprocess.set_enabled(
        "vignette",
        args.iter().any(
            |arg| arg == "--vignette"
        ),
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
        nes.update_input(
            &window
        );

        nes.run_frame();

        let samples =
            nes.take_audio_samples();

        audio.push_samples(
            &samples
        );

        postprocess.apply(
            nes.framebuffer_mut(),
            WIDTH,
            HEIGHT,
        );

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
