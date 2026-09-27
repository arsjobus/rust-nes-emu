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

const INITIAL_SCALE: usize = 3;

const BORDER_COLOR: u32 = 0x000000;

pub fn run(
mut nes: Nes,
args: &[String],
) {
/*
* ---------------------------------------------------------
* Window
* ---------------------------------------------------------
*
* The NES framebuffer remains 256x240.
*
* The actual window is resizable. We scale the framebuffer
* ourselves so resizing/maximizing does not affect the
* emulator or PPU.
* ---------------------------------------------------------
*/

let initial_width =
    WIDTH * INITIAL_SCALE;

let initial_height =
    HEIGHT * INITIAL_SCALE;

let mut window =
    Window::new(
        "NES",
        initial_width,
        initial_height,
        WindowOptions {
            resize: true,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect(
        "Could not create window"
    );

window.set_target_fps(60);

/*
 * ---------------------------------------------------------
 * Display framebuffer
 * ---------------------------------------------------------
 *
 * This is separate from the PPU framebuffer.
 *
 * PPU:
 *
 *     256 x 240
 *
 * Display:
 *
 *     Whatever size the window currently is.
 * ---------------------------------------------------------
 */

let mut display_buffer =
    vec![
        BORDER_COLOR;
        initial_width *
        initial_height
    ];

/*
 * ---------------------------------------------------------
 * Audio
 * ---------------------------------------------------------
 */

let mut audio =
    Audio::new()
        .expect(
            "Could not initialize audio"
        );

/*
 * ---------------------------------------------------------
 * Post-processing pipeline
 * ---------------------------------------------------------
 */

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

/*
 * ---------------------------------------------------------
 * Maximum frame count
 * ---------------------------------------------------------
 */

let max_frames =
    env::var("NES_MAX_FRAMES")
        .ok()
        .and_then(
            |v| v.parse::<u64>().ok()
        );

let mut frame_count =
    0u64;

/*
 * ---------------------------------------------------------
 * Frame timing
 * ---------------------------------------------------------
 */

let mut last =
    Instant::now();

/*
 * ---------------------------------------------------------
 * Main loop
 * ---------------------------------------------------------
 */

while window.is_open()
    && !window.is_key_down(
        Key::Escape
    )
{
    /*
     * -----------------------------------------------------
     * Input
     * -----------------------------------------------------
     */

    nes.update_input(
        &window
    );

    /*
     * -----------------------------------------------------
     * Run one NES frame.
     * -----------------------------------------------------
     */

    nes.run_frame();

    /*
     * -----------------------------------------------------
     * Audio
     * -----------------------------------------------------
     */

    let samples =
        nes.take_audio_samples();

    audio.push_samples(
        &samples
    );

    /*
     * -----------------------------------------------------
     * Post-processing
     *
     * Always process the native 256x240 framebuffer.
     * Scaling happens afterwards.
     * -----------------------------------------------------
     */

    postprocess.apply(
        nes.framebuffer_mut(),
        WIDTH,
        HEIGHT,
    );

    /*
     * -----------------------------------------------------
     * Get current window size.
     * -----------------------------------------------------
     */

    let (
        window_width,
        window_height,
    ) = window.get_size();

    /*
     * -----------------------------------------------------
     * Reallocate display buffer only when necessary.
     * -----------------------------------------------------
     */

    let required_size =
        window_width *
        window_height;

    if display_buffer.len()
        != required_size
    {
        display_buffer =
            vec![
                BORDER_COLOR;
                required_size
            ];
    }

    /*
     * -----------------------------------------------------
     * Clear the window.
     * -----------------------------------------------------
     */

    display_buffer.fill(
        BORDER_COLOR
    );

    /*
     * -----------------------------------------------------
     * Calculate scaling.
     *
     * We preserve the NES framebuffer's native aspect
     * ratio of 256:240.
     *
     * The image is enlarged as much as possible while
     * remaining completely visible.
     *
     * We use integer scaling whenever possible.
     * -----------------------------------------------------
     */

    let integer_scale =
        (window_width / WIDTH)
            .min(
                window_height / HEIGHT
            );

    let (
        scaled_width,
        scaled_height,
    ) = if integer_scale >= 1 {
        (
            WIDTH * integer_scale,
            HEIGHT * integer_scale,
        )
    } else {
        /*
         * Extremely small window.
         *
         * Allow fractional scaling so the game remains
         * visible instead of disappearing.
         */

        let scale_x =
            window_width as f32
                / WIDTH as f32;

        let scale_y =
            window_height as f32
                / HEIGHT as f32;

        let scale =
            scale_x.min(scale_y);

        (
            (WIDTH as f32 * scale)
                as usize,
            (HEIGHT as f32 * scale)
                as usize,
        )
    };

    if scaled_width == 0 ||
       scaled_height == 0
    {
        continue;
    }

    /*
     * -----------------------------------------------------
     * Center the game image.
     * -----------------------------------------------------
     */

    let offset_x =
        (window_width
            - scaled_width)
            / 2;

    let offset_y =
        (window_height
            - scaled_height)
            / 2;

    /*
     * -----------------------------------------------------
     * Nearest-neighbor scaling.
     *
     * This keeps NES pixels sharp instead of applying
     * interpolation/blur.
     * -----------------------------------------------------
     */

    for dest_y in
        0..scaled_height
    {
        let source_y =
            dest_y *
            HEIGHT /
            scaled_height;

        let window_y =
            offset_y +
            dest_y;

        if window_y >=
            window_height
        {
            continue;
        }

        for dest_x in
            0..scaled_width
        {
            let source_x =
                dest_x *
                WIDTH /
                scaled_width;

            let window_x =
                offset_x +
                dest_x;

            if window_x >=
                window_width
            {
                continue;
            }

            let source_index =
                source_y *
                WIDTH +
                source_x;

            let destination_index =
                window_y *
                window_width +
                window_x;

            display_buffer[
                destination_index
            ] = nes.framebuffer()[
                source_index
            ];
        }
    }

    /*
     * -----------------------------------------------------
     * Present frame.
     * -----------------------------------------------------
     */

    window
        .update_with_buffer(
            &display_buffer,
            window_width,
            window_height,
        )
        .expect(
            "Failed to update window"
        );

    /*
     * -----------------------------------------------------
     * Frame counter.
     * -----------------------------------------------------
     */

    frame_count += 1;

    if let Some(max) =
        max_frames
    {
        if frame_count >= max {
            break;
        }
    }

    /*
     * -----------------------------------------------------
     * 60 FPS frame pacing.
     * -----------------------------------------------------
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