#[cfg(test)]
mod rom_harness;
mod apu;
mod audio;
mod bus;
mod cartridge;
mod cpu;
mod input;
mod nes;
mod postprocess;
mod ppu;
mod recorder;
mod video;

use std::env;

use crate::{cartridge::Cartridge, nes::Nes};

fn rom_argument(args: &[String]) -> Option<&str> {
    const VALUE_OPTIONS: &[&str] = &[
        "--ntsc-strength",
        "--ntsc-bleed",
        "--persistence-amount",
        "--persistence-frames",
        "--bloom-strength",
        "--bloom-threshold",
        "--bloom-radius",
        "--color-brightness",
        "--color-contrast",
        "--color-saturation",
        "--color-gamma",
        "--lut-strength",
        "--curvature-strength",
        "--auto-gradient-strength",
        "--auto-gradient-vertical",
        "--auto-gradient-horizontal",
        "--scanlines-strength",
        "--vignette-strength",
        "--crt-strength",
    ];
    let mut skip_value = false;
    for arg in args.iter().skip(1) {
        if skip_value {
            skip_value = false;
            continue;
        }
        if VALUE_OPTIONS.contains(&arg.as_str()) {
            skip_value = true;
        } else if arg == "--lut" {
            // LUT preset is optional; recognize a preset token if supplied separately.
            continue;
        } else if !arg.starts_with('-') {
            let preset = matches!(
                arg.as_str(),
                "identity"
                    | "warm-crt"
                    | "cool-crt"
                    | "composite"
                    | "gameboy"
                    | "game-boy"
                    | "amber"
                    | "high-contrast"
            );
            if preset {
                continue;
            }
            return Some(arg);
        }
    }
    None
}

fn print_usage() {
    eprintln!("Usage: runes [rom.nes] [options]");
    eprintln!();
    eprintln!("Post-processing options:");
    eprintln!("  --ntsc                  Enable NTSC/composite color bleed");
    eprintln!("  --persistence           Enable motion persistence");
    eprintln!("  --bloom                 Enable bloom");
    eprintln!("  --bloom-strength <0-1>  Bloom intensity (default: 0.20)");
    eprintln!("  --bloom-threshold <0-255>  Brightness threshold (default: 180)");
    eprintln!("  --bloom-radius <n>      Bloom spread radius (default: 3)");
    eprintln!("  --color-correction      Enable color correction");
    eprintln!(
        "  --lut [preset]          Enable LUT: warm-crt, cool-crt, composite, gameboy, amber, high-contrast, identity"
    );
    eprintln!("  --lut-strength <0-1>    LUT blend strength (default: 0.65)");
    eprintln!("  --curvature             Enable CRT curvature");
    eprintln!("  --auto-gradient         Enable automatic screen gradient");
    eprintln!("  --scanlines             Enable scanlines");
    eprintln!("  --vignette              Enable vignette");
    eprintln!("  --crt                   Enable CRT phosphor/scanline effect");
    eprintln!("  --crt-strength <0-1>    CRT effect intensity (default: 0.75)");
    eprintln!("  --<effect>-strength <n> Configure effect strength where available");
    eprintln!("  Values can be passed as --option value or --option=value.");
    eprintln!();
    eprintln!("Controller:");
    eprintln!("  USB gamepad             Logitech/gamepad controller support");
    eprintln!(
        "  Right trigger           Start/stop MP4 video and audio recording (requires ffmpeg)"
    );
    eprintln!("  0                       Start/stop MP4 video and audio recording");
    eprintln!("  Left Shift              Toggle turbo for held A/B buttons");
    eprintln!("  Left trigger            Toggle turbo for held A/B buttons");
    eprintln!("  Controller X            Toggle CRT effect while playing");
    eprintln!("  Hold Start for 3 seconds Return to the ROM menu");
    eprintln!();
    eprintln!("Idle screen:");
    eprintln!("  runes                   Show TV static and choose a ROM");
    eprintln!("  Up/Down or D-pad        Navigate the ROM list");
    eprintln!("  Enter, gamepad A/B/Start Launch the selected ROM");
    eprintln!("  O                       Rescan current folder and roms/");
    eprintln!("  R                       Return from a game to the ROM menu");
    eprintln!();
    eprintln!("Other options:");
    eprintln!("  --full-screen           Start in desktop full-screen mode");
    eprintln!("  --help                  Show this help");
    eprintln!();
    eprintln!("Environment variables:");
    eprintln!("  NES_MAX_FRAMES=<n>      Stop after <n> frames");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  runes game.nes");
    eprintln!("  runes game.nes --scanlines");
    eprintln!("  runes game.nes --ntsc --scanlines --curvature");
    eprintln!("  runes game.nes --bloom --vignette --lut");
    eprintln!("  runes game.nes --bloom --bloom-strength 0.4 --lut cool-crt --lut-strength 0.8");
    eprintln!("  runes game.nes --ntsc --persistence --bloom --color-correction \\");
    eprintln!("      --lut --curvature --auto-gradient --scanlines --vignette \\");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    let rom_path = rom_argument(&args);
    let nes = if let Some(rom_path) = rom_path {
        match Cartridge::load(rom_path) {
            Ok(cart) => Some(Nes::new(cart)),
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        }
    } else {
        None
    };

    /*
     * ---------------------------------------------------------
     * Start emulator
     * ---------------------------------------------------------
     *
     * video::run() handles:
     *
     *   - window events
     *   - keyboard input
     *   - USB controller polling
     *   - NES frame execution
     *   - audio
     *   - post-processing
     *   - framebuffer scaling
     * ---------------------------------------------------------
     */

    video::run(nes, &args);
}
