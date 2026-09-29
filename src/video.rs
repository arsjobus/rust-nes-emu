use gilrs::{Button, EventType, Gilrs};
use sdl2::{
    event::Event, keyboard::Scancode, pixels::PixelFormatEnum, rect::Rect, render::ScaleMode,
};
use std::{fs, path::PathBuf};

use std::{
    env,
    time::{Duration, Instant},
};

use crate::nes::Nes;
use crate::{audio::Audio, cartridge::Cartridge};

use crate::postprocess::effect::PostProcessEffect;
use crate::postprocess::{
    AutoGradient, Bloom, ColorCorrection, Crt, Curvature, Lut, LutPreset, Ntsc, Persistence,
    PostProcessPipeline, Scanlines, Vignette,
};

pub(crate) const WIDTH: usize = 256;
pub(crate) const HEIGHT: usize = 240;

const INITIAL_SCALE: usize = 3;
const CRT_SCALE: usize = 3;

fn option_value<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| {
            arg.strip_prefix(&prefix)
                .or_else(|| (arg == name).then(|| ""))
                .and_then(|value| {
                    if value.is_empty() {
                        None
                    } else {
                        value.parse().ok()
                    }
                })
                .or_else(|| {
                    (arg == name)
                        .then(|| args.iter().position(|candidate| candidate == name))
                        .flatten()
                        .and_then(|index| args.get(index + 1))
                        .and_then(|value| value.parse().ok())
                })
        })
        .unwrap_or(default)
}

fn lut_preset(args: &[String]) -> LutPreset {
    let name: String = option_value(args, "--lut", String::from("warm-crt"));
    match name.to_ascii_lowercase().as_str() {
        "identity" => LutPreset::Identity,
        "cool" | "cool-crt" => LutPreset::CoolCrt,
        "composite" => LutPreset::Composite,
        "gameboy" | "game-boy" => LutPreset::GameBoy,
        "amber" => LutPreset::Amber,
        "high-contrast" | "highcontrast" => LutPreset::HighContrast,
        _ => LutPreset::WarmCrt,
    }
}

pub fn run(mut nes: Option<Nes>, args: &[String]) {
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

    let sdl = sdl2::init().expect("Could not initialize SDL");
    let video = sdl.video().expect("Could not initialize SDL video");
    let audio_subsystem = sdl.audio().ok();
    let window = video
        .window(
            "RuNES",
            (WIDTH * INITIAL_SCALE) as u32,
            (HEIGHT * INITIAL_SCALE) as u32,
        )
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .expect("Could not create window");
    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .or_else(|_| {
            video
                .window(
                    "RuNES",
                    (WIDTH * INITIAL_SCALE) as u32,
                    (HEIGHT * INITIAL_SCALE) as u32,
                )
                .position_centered()
                .resizable()
                .allow_highdpi()
                .build()
                .unwrap()
                .into_canvas()
                .software()
                .build()
        })
        .expect("Could not create SDL renderer");
    let texture_creator = canvas.texture_creator();
    let output_width = WIDTH * CRT_SCALE;
    let output_height = HEIGHT * CRT_SCALE;
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGBA32,
            output_width as u32,
            output_height as u32,
        )
        .expect("Could not create framebuffer texture");
    texture.set_scale_mode(ScaleMode::Nearest);
    let mut event_pump = sdl.event_pump().expect("Could not create SDL event pump");
    let mut rgba_buffer = vec![0u8; output_width * output_height * 4];
    let mut output_framebuffer = vec![0u32; output_width * output_height];
    let mut audio: Option<Audio> = if nes.is_some() {
        audio_subsystem
            .as_ref()
            .and_then(|subsystem| Audio::new(subsystem).ok())
    } else {
        None
    };
    if let (Some(audio), Some(nes)) = (audio.as_ref(), nes.as_mut()) {
        nes.bus.apu.set_sample_rate(audio.sample_rate() as f64);
    }

    /*
     * ---------------------------------------------------------
     * Post-processing pipeline
     * ---------------------------------------------------------
     */

    let mut postprocess = PostProcessPipeline::new();

    /*
     * ---------------------------------------------------------
     * NTSC
     * ---------------------------------------------------------
     */

    postprocess.add(Ntsc::new(
        option_value(args, "--ntsc-strength", 0.65),
        option_value(args, "--ntsc-bleed", 2),
    ));

    postprocess.set_enabled("ntsc", args.iter().any(|arg| arg == "--ntsc"));

    /*
     * ---------------------------------------------------------
     * Persistence
     * ---------------------------------------------------------
     */

    postprocess.add(Persistence::new(
        option_value(args, "--persistence-amount", 0.2),
        option_value(args, "--persistence-frames", 3),
    ));

    postprocess.set_enabled("persistence", args.iter().any(|arg| arg == "--persistence"));

    /*
     * ---------------------------------------------------------
     * Bloom
     * ---------------------------------------------------------
     */

    postprocess.add(Bloom::new(
        option_value(args, "--bloom-threshold", 180),
        option_value(args, "--bloom-strength", 0.20),
        option_value(args, "--bloom-radius", 3),
    ));

    postprocess.set_enabled("bloom", args.iter().any(|arg| arg == "--bloom"));

    /*
     * ---------------------------------------------------------
     * Color correction
     * ---------------------------------------------------------
     */

    postprocess.add(ColorCorrection::new(
        option_value(args, "--color-brightness", 0.0),
        option_value(args, "--color-contrast", 1.05),
        option_value(args, "--color-saturation", 0.95),
        option_value(args, "--color-gamma", 1.0),
    ));

    postprocess.set_enabled(
        "color_correction",
        args.iter().any(|arg| arg == "--color-correction"),
    );

    /*
     * ---------------------------------------------------------
     * LUT
     * ---------------------------------------------------------
     */

    postprocess.add(Lut::new(
        lut_preset(args),
        option_value(args, "--lut-strength", 0.65),
    ));

    postprocess.set_enabled(
        "lut",
        args.iter()
            .any(|arg| arg == "--lut" || arg.starts_with("--lut=")),
    );

    /*
     * ---------------------------------------------------------
     * CRT curvature
     * ---------------------------------------------------------
     */

    postprocess.add(Curvature::new(option_value(
        args,
        "--curvature-strength",
        0.05,
    )));

    postprocess.set_enabled("curvature", args.iter().any(|arg| arg == "--curvature"));

    /*
     * ---------------------------------------------------------
     * Automatic gradient
     * ---------------------------------------------------------
     */

    postprocess.add(AutoGradient::new(
        option_value(args, "--auto-gradient-strength", 0.15),
        option_value(args, "--auto-gradient-vertical", 0.40),
        option_value(args, "--auto-gradient-horizontal", 0.25),
    ));

    postprocess.set_enabled(
        "auto_gradient",
        args.iter().any(|arg| arg == "--auto-gradient"),
    );

    /*
     * ---------------------------------------------------------
     * Scanlines
     * ---------------------------------------------------------
     */

    postprocess.add(Scanlines::new(option_value(
        args,
        "--scanlines-strength",
        0.1,
    )));

    postprocess.set_enabled("scanlines", args.iter().any(|arg| arg == "--scanlines"));

    /*
     * ---------------------------------------------------------
     * Vignette
     * ---------------------------------------------------------
     */

    postprocess.add(Vignette::new(option_value(
        args,
        "--vignette-strength",
        0.35,
    )));

    postprocess.set_enabled("vignette", args.iter().any(|arg| arg == "--vignette"));

    // CRT shader-style treatment, toggled at runtime with the controller X button.
    let mut crt = Crt::new(option_value(args, "--crt-strength", 0.75));
    crt.set_enabled(args.iter().any(|arg| arg == "--crt"));

    /*
     * ---------------------------------------------------------
     * Maximum frame count
     * ---------------------------------------------------------
     */

    let max_frames = env::var("NES_MAX_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());

    let mut frame_count = 0u64;
    let mut noise_seed = 0x5eed_u32;
    let mut idle_framebuffer = vec![0; WIDTH * HEIGHT];
    let mut roms = find_roms();
    let mut selection = 0usize;
    let mut previous_up = false;
    let mut previous_down = false;
    let mut repeat_up_at = None;
    let mut repeat_down_at = None;
    let mut previous_enter = false;
    let mut previous_o = false;
    let mut menu_gamepad = if nes.is_none() {
        Gilrs::new().ok()
    } else {
        None
    };
    let mut previous_pad_up = false;
    let mut previous_pad_down = false;
    let mut previous_pad_select = false;
    let mut previous_reset = false;
    let mut recorder: Option<crate::recorder::Recorder> = None;
    let mut lut_notice: Option<(String, Instant)> = None;

    /*
     * ---------------------------------------------------------
     * Frame timing
     * ---------------------------------------------------------
     */

    let mut last = Instant::now();

    /*
     * ---------------------------------------------------------
     * Main loop
     * ---------------------------------------------------------
     */

    let mut running = true;
    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => running = false,
                _ => {}
            }
        }
        if !running {
            break;
        }
        let keyboard = event_pump.keyboard_state();
        let up = keyboard.is_scancode_pressed(Scancode::Up);
        let down = keyboard.is_scancode_pressed(Scancode::Down);
        let enter = keyboard.is_scancode_pressed(Scancode::Return);
        let open = keyboard.is_scancode_pressed(Scancode::O);
        let reset = keyboard.is_scancode_pressed(Scancode::R);

        if nes.is_some() && reset && !previous_reset {
            if let Some(current) = nes.as_ref() {
                if let Err(error) = current.save_battery_ram() {
                    eprintln!("Could not save battery RAM: {}", error);
                }
            }
            nes = None;
            roms = find_roms();
            selection = 0;
            menu_gamepad = Gilrs::new().ok();
        }
        previous_reset = reset;

        let (pad_up, pad_down, pad_select) = menu_gamepad_state(&mut menu_gamepad);

        if nes.is_none() {
            let now = Instant::now();
            let move_up = menu_navigation_pressed(
                up || pad_up,
                previous_up || previous_pad_up,
                &mut repeat_up_at,
                now,
            );
            let move_down = menu_navigation_pressed(
                down || pad_down,
                previous_down || previous_pad_down,
                &mut repeat_down_at,
                now,
            );
            if move_up && !roms.is_empty() {
                selection = (selection + roms.len() - 1) % roms.len();
            }
            if move_down && !roms.is_empty() {
                selection = (selection + 1) % roms.len();
            }
            if open && !previous_o {
                roms = find_roms();
                selection = 0;
            }
            if (enter && !previous_enter) || (pad_select && !previous_pad_select) {
                if let Some(path) = roms.get(selection) {
                    match Cartridge::load(&path.to_string_lossy()) {
                        Ok(cart) => {
                            if let Some(current) = nes.as_ref() {
                                if let Err(error) = current.save_battery_ram() {
                                    eprintln!("Could not save battery RAM: {}", error);
                                }
                            }
                            nes = Some(Nes::new(cart));
                            menu_gamepad = None;
                            if audio.is_none() {
                                audio = audio_subsystem
                                    .as_ref()
                                    .and_then(|subsystem| Audio::new(subsystem).ok());
                                if let (Some(audio), Some(nes)) = (audio.as_ref(), nes.as_mut()) {
                                    nes.bus.apu.set_sample_rate(audio.sample_rate() as f64);
                                }
                            }
                        }
                        Err(error) => eprintln!("Could not load {}: {}", path.display(), error),
                    }
                }
            }
        }
        previous_up = up;
        previous_down = down;
        previous_enter = enter;
        previous_o = open;
        previous_pad_up = pad_up;
        previous_pad_down = pad_down;
        previous_pad_select = pad_select;
        if let Some(nes) = nes.as_mut() {
            nes.update_input(&keyboard);
            if nes.bus.controller.take_record_toggle() {
                if let Some(active) = recorder.take() {
                    match active.finish() {
                        Ok(path) => println!("Recording saved: {}", path.display()),
                        Err(error) => eprintln!("Could not finish recording: {error}"),
                    }
                } else {
                    let sample_rate = audio.as_ref().map_or(48_000, Audio::sample_rate);
                    match crate::recorder::Recorder::start(sample_rate) {
                        Ok(started) => {
                            println!("Recording started");
                            recorder = Some(started);
                        }
                        Err(error) => eprintln!("Could not start recording: {error}"),
                    }
                }
            }
            if nes.bus.controller.take_color_cycle_toggle() {
                postprocess.set_enabled("color_correction", true);
                postprocess.set_enabled("lut", true);
                postprocess.cycle_lut();
                if let Some(label) = postprocess.lut_label() {
                    lut_notice = Some((label.to_string(), Instant::now()));
                }
            }
            if nes.bus.controller.take_crt_toggle() {
                let enabled = !crt.enabled();
                crt.set_enabled(enabled);
                lut_notice = Some((
                    format!("CRT {}", if enabled { "ON" } else { "OFF" }),
                    Instant::now(),
                ));
            }
            nes.run_frame();
            let samples = nes.take_audio_samples();
            if let Some(audio) = audio.as_mut() {
                audio.push_samples(&samples);
            }
            let audio_error = recorder
                .as_mut()
                .and_then(|recorder| recorder.write_audio(&samples).err());
            if let Some(error) = audio_error {
                eprintln!("Recording audio failed: {error}");
                recorder = None;
            }
            postprocess.apply(nes.framebuffer_mut(), WIDTH, HEIGHT);
            if nes.turbo_enabled() {
                draw_text(
                    nes.framebuffer_mut(),
                    WIDTH,
                    HEIGHT,
                    2,
                    2,
                    "TURBO",
                    1,
                    0xffffff,
                );
            }
            if lut_notice
                .as_ref()
                .is_some_and(|(_, shown_at)| shown_at.elapsed() >= Duration::from_secs(3))
            {
                lut_notice = None;
            }
            if let Some((label, shown_at)) = &lut_notice {
                let elapsed = shown_at.elapsed();
                if elapsed < Duration::from_secs(3) {
                    let alpha = if elapsed <= Duration::from_secs(1) {
                        1.0
                    } else {
                        1.0 - (elapsed.as_secs_f32() - 1.0) / 2.0
                    };
                    draw_text_faded(
                        nes.framebuffer_mut(),
                        WIDTH,
                        HEIGHT,
                        label,
                        alpha.clamp(0.0, 1.0),
                    );
                }
            }
        }

        if nes.is_none() {
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    idle_framebuffer[y * WIDTH + x] =
                        static_pixel(&mut noise_seed, x, y, frame_count);
                }
            }
            draw_idle_overlay(&mut idle_framebuffer, WIDTH, HEIGHT, &roms, selection);
        }

        let source = nes
            .as_ref()
            .map_or(idle_framebuffer.as_slice(), Nes::framebuffer);
        let video_error = recorder
            .as_mut()
            .and_then(|recorder| recorder.write_frame(source).err());
        if let Some(error) = video_error {
            eprintln!("Recording video failed: {error}");
            recorder = None;
        }
        for y in 0..output_height {
            let source_y = y / CRT_SCALE;
            for x in 0..output_width {
                output_framebuffer[y * output_width + x] = source[source_y * WIDTH + x / CRT_SCALE];
            }
        }
        if nes.is_some() && crt.enabled() {
            crt.apply(&mut output_framebuffer, output_width, output_height);
        }
        for (pixel, rgba) in output_framebuffer
            .iter()
            .zip(rgba_buffer.chunks_exact_mut(4))
        {
            rgba[0] = (pixel >> 16) as u8;
            rgba[1] = (pixel >> 8) as u8;
            rgba[2] = *pixel as u8;
            rgba[3] = 255;
        }
        texture
            .update(None, &rgba_buffer, output_width * 4)
            .expect("Failed to update framebuffer texture");
        let (window_width, window_height) = canvas.output_size().unwrap_or((0, 0));
        if window_width > 0 && window_height > 0 {
            let scale =
                (window_width as f64 / WIDTH as f64).min(window_height as f64 / HEIGHT as f64);
            let scaled_width = (WIDTH as f64 * scale) as u32;
            let scaled_height = (HEIGHT as f64 * scale) as u32;
            let destination = Rect::new(
                ((window_width - scaled_width) / 2) as i32,
                ((window_height - scaled_height) / 2) as i32,
                scaled_width,
                scaled_height,
            );
            canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
            canvas.clear();
            canvas
                .copy(&texture, None, destination)
                .expect("Failed to render framebuffer");
            if recorder.is_some() {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 0, 0));
                let border = (scale.round() as u32).max(2);
                for inset in 0..border {
                    let x = destination.x() + inset as i32;
                    let y = destination.y() + inset as i32;
                    let width = destination.width().saturating_sub(inset * 2);
                    let height = destination.height().saturating_sub(inset * 2);
                    if width > 0 && height > 0 {
                        let _ = canvas.draw_rect(Rect::new(x, y, width, height));
                    }
                }
            }
            canvas.present();
        }

        /*
         * -----------------------------------------------------
         * Frame counter.
         * -----------------------------------------------------
         */

        frame_count += 1;

        if let Some(max) = max_frames {
            if frame_count >= max {
                break;
            }
        }

        /*
         * -----------------------------------------------------
         * 60 FPS frame pacing.
         * -----------------------------------------------------
         */

        let elapsed = last.elapsed();

        if elapsed < Duration::from_micros(16_667) {
            std::thread::sleep(Duration::from_micros(16_667) - elapsed);
        }

        last = Instant::now();
    }

    if let Some(current) = nes.as_ref() {
        if let Err(error) = current.save_battery_ram() {
            eprintln!("Could not save battery RAM: {}", error);
        }
    }
    if let Some(active) = recorder {
        match active.finish() {
            Ok(path) => println!("Recording saved: {}", path.display()),
            Err(error) => eprintln!("Could not finish recording: {error}"),
        }
    }
}

fn find_roms() -> Vec<PathBuf> {
    let mut roms = Vec::new();
    if let Ok(entries) = fs::read_dir(".") {
        roms.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("nes"))
        }));
    }
    let mut pending = vec![PathBuf::from("roms")];
    while let Some(directory) = pending.pop() {
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else if path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("nes"))
                {
                    roms.push(path);
                }
            }
        }
    }
    roms.sort();
    roms.dedup();
    roms
}

fn menu_navigation_pressed(
    pressed: bool,
    was_pressed: bool,
    repeat_at: &mut Option<Instant>,
    now: Instant,
) -> bool {
    const INITIAL_REPEAT_DELAY: Duration = Duration::from_millis(350);
    const REPEAT_INTERVAL: Duration = Duration::from_millis(100);

    if !pressed {
        *repeat_at = None;
        return false;
    }
    if !was_pressed {
        *repeat_at = Some(now + INITIAL_REPEAT_DELAY);
        return true;
    }
    if let Some(next_repeat) = *repeat_at {
        if now >= next_repeat {
            *repeat_at = Some(now + REPEAT_INTERVAL);
            return true;
        }
    }
    false
}

fn menu_gamepad_state(gilrs: &mut Option<Gilrs>) -> (bool, bool, bool) {
    let Some(gilrs) = gilrs.as_mut() else {
        return (false, false, false);
    };
    while let Some(event) = gilrs.next_event() {
        if let EventType::Connected = event.event {
            let gamepad = gilrs.gamepad(event.id);
            println!("Controller connected: {}", gamepad.name());
        }
    }

    gilrs
        .gamepads()
        .next()
        .map_or((false, false, false), |(_, gamepad)| {
            let up =
                gamepad.is_pressed(Button::DPadUp) || gamepad.value(gilrs::Axis::LeftStickY) < -0.5;
            let down = gamepad.is_pressed(Button::DPadDown)
                || gamepad.value(gilrs::Axis::LeftStickY) > 0.5;
            let select = gamepad.is_pressed(Button::South)
                || gamepad.is_pressed(Button::East)
                || gamepad.is_pressed(Button::Start);
            (up, down, select)
        })
}

fn static_pixel(seed: &mut u32, x: usize, y: usize, frame: u64) -> u32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    let grain = ((*seed >> 24) & 0xff) as i32;
    let scanline = if (y + frame as usize / 3) % 4 == 0 {
        12
    } else {
        0
    };
    let vignette = ((x.abs_diff(WIDTH / 2) + y.abs_diff(HEIGHT / 2)) / 24).min(24) as i32;
    let level = (grain / 2 + scanline - vignette).clamp(0, 115) as u32;
    (level << 16) | (level << 8) | level
}

fn draw_idle_overlay(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    roms: &[PathBuf],
    selection: usize,
) {
    // A compact 5x7 bitmap font keeps the idle screen self-contained.
    let first_visible = selection
        .saturating_sub(7)
        .min(roms.len().saturating_sub(8));
    let lines: Vec<String> = if roms.is_empty() {
        vec![
            "RUNES - NO ROMS FOUND".into(),
            "ADD .NES FILES TO THIS FOLDER OR ROMS/".into(),
            "PRESS O TO RESCAN".into(),
            "ESC TO QUIT".into(),
        ]
    } else {
        let mut lines = vec![
            "RUNES - SELECT A GAME".into(),
            "DPAD/STICK: MOVE".into(),
            "A/START: OPEN  R IN GAME: MENU".into(),
            "O: RESCAN".into(),
        ];
        for (visible_index, path) in roms.iter().enumerate().skip(first_visible).take(8) {
            let i = visible_index;
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let name: String = name.chars().take(26).collect();
            lines.push(format!(
                "{} {}",
                if i == selection { ">" } else { " " },
                name.to_uppercase()
            ));
        }
        lines
    };
    let scale = (width / 480).clamp(1, 3);
    let line_height = 9 * scale;
    let panel_h = (lines.len() * line_height + 20 * scale).min(height);
    let panel_w = width * 3 / 4;
    let left = (width - panel_w) / 2;
    let top = (height - panel_h) / 2;
    for y in top..(top + panel_h).min(height) {
        for x in left..(left + panel_w).min(width) {
            let i = y * width + x;
            buffer[i] = (buffer[i] & 0x3f3f3f) / 2;
        }
    }
    for (row, line) in lines.iter().enumerate() {
        let y = top + 10 * scale + row * line_height;
        draw_text(
            buffer,
            width,
            height,
            left + 10 * scale,
            y,
            line,
            scale,
            if row >= 4 && first_visible + row - 4 == selection {
                0x00ff88
            } else {
                0xffffff
            },
        );
    }
}

fn draw_text(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    color: u32,
) {
    for (column, ch) in text.chars().enumerate() {
        let glyph = glyph(ch);
        for (gy, row) in glyph.iter().enumerate() {
            for gx in 0..5 {
                if row & (1 << (4 - gx)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = x + (column * 6 + gx) * scale + sx;
                            let py = y + gy * scale + sy;
                            if px < width && py < height {
                                buffer[py * width + px] = color;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn draw_text_faded(buffer: &mut [u32], width: usize, height: usize, text: &str, alpha: f32) {
    let scale = 1;
    let text_width = text.chars().count() * 6 * scale - scale;
    let x = width.saturating_sub(text_width) / 2;
    let y = 8;
    let alpha = (alpha * 255.0).round() as u32;
    for (column, ch) in text.chars().enumerate() {
        for (gy, row) in glyph(ch).iter().enumerate() {
            for gx in 0..5 {
                if row & (1 << (4 - gx)) == 0 {
                    continue;
                }
                let px = x + column * 6 + gx;
                let py = y + gy;
                if px < width && py < height {
                    let index = py * width + px;
                    let pixel = buffer[index];
                    let r = ((pixel >> 16) & 0xff) * (255 - alpha) / 255 + alpha;
                    let g = ((pixel >> 8) & 0xff) * (255 - alpha) / 255 + alpha;
                    let b = (pixel & 0xff) * (255 - alpha) / 255 + alpha;
                    buffer[index] = (r << 16) | (g << 8) | b;
                }
            }
        }
    }
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '>' => [16, 8, 4, 2, 4, 8, 16],
        _ => [0; 7],
    }
}
