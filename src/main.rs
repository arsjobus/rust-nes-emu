mod apu;
mod audio;
mod bus;
mod cartridge;
mod cpu;
mod input;
mod nes;
mod postprocess;
mod ppu;
mod video;

use std::env;

use crate::{cartridge::Cartridge, nes::Nes};

fn print_usage() {
    eprintln!("Usage: runes [rom.nes] [options]");
    eprintln!();
    eprintln!("Post-processing options:");
    eprintln!("  --ntsc                  Enable NTSC/composite color bleed");
    eprintln!("  --persistence           Enable motion persistence");
    eprintln!("  --bloom                 Enable bloom");
    eprintln!("  --color-correction      Enable color correction");
    eprintln!("  --lut                   Enable LUT color grading");
    eprintln!("  --curvature             Enable CRT curvature");
    eprintln!("  --auto-gradient         Enable automatic screen gradient");
    eprintln!("  --scanlines             Enable scanlines");
    eprintln!("  --vignette              Enable vignette");
    eprintln!();
    eprintln!("Controller:");
    eprintln!("  USB gamepad             Logitech/gamepad controller support");
    eprintln!();
    eprintln!("Idle screen:");
    eprintln!("  runes                   Show TV static and choose a ROM");
    eprintln!("  Up/Down or D-pad        Navigate the ROM list");
    eprintln!("  Enter, gamepad A/Start  Launch the selected ROM");
    eprintln!("  O                       Rescan current folder and roms/");
    eprintln!("  R                       Return from a game to the ROM menu");
    eprintln!();
    eprintln!("Other options:");
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
    eprintln!("  runes game.nes --ntsc --persistence --bloom --color-correction \\");
    eprintln!("      --lut --curvature --auto-gradient --scanlines --vignette \\");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    let rom_path = args.iter().skip(1).find(|arg| !arg.starts_with('-'));
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
