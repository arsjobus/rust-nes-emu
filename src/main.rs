mod bus;
mod cartridge;
mod cpu;
mod input;
mod nes;
mod ppu;
mod video;

use std::env;

use cartridge::Cartridge;
use input::Controller;
use nes::Nes;
use video::Video;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: nes <rom.nes>");
        return;
    }

    let rom = &args[1];

    let cart = match Cartridge::load(rom) {
        Ok(cart) => cart,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let controller = Controller::new();

    let mut nes = Nes::new(cart, controller);

    nes.reset();

    let mut video = Video::new();

    let max_frames = env::var("NES_MAX_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());

    video.run(&mut nes, max_frames);
}
