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

use crate::{
    cartridge::Cartridge,
    nes::Nes,
    ppu::PpuOptions,
};

fn print_usage() {
    eprintln!("Usage: runes <rom.nes> [options]");
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
    eprintln!("  --sprite-shadows        Enable sprite shadows");
    eprintln!();
    eprintln!("Controller:");
    eprintln!("  USB gamepad             Logitech/gamepad controller support");
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
    eprintln!("      --sprite-shadows");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    /*
     * ---------------------------------------------------------
     * ROM
     * ---------------------------------------------------------
     */

    let rom_path = &args[1];

    let cart = match Cartridge::load(rom_path) {
        Ok(cart) => cart,

        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    /*
     * ---------------------------------------------------------
     * NES
     * ---------------------------------------------------------
     *
     * The NES owns the Bus.
     *
     * The Bus owns the Controller.
     *
     * The Controller owns the gilrs USB gamepad interface.
     *
     * Therefore main.rs does not need to create an Input
     * object separately.
     * ---------------------------------------------------------
     */

    let mut nes = Nes::new(cart);

    /*
     * ---------------------------------------------------------
     * PPU options
     * ---------------------------------------------------------
     */

    let sprite_shadows = args
        .iter()
        .any(|arg| arg == "--sprite-shadows");

    nes.bus.ppu.options = PpuOptions {
        sprite_shadows,
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

    video::run(
        nes,
        &args,
    );
}
