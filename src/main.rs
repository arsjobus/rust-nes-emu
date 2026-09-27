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
};

fn main() {
    let args: Vec<String> =
        env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: nes <rom.nes>");
        return;
    }

    let cart =
        match Cartridge::load(&args[1]) {
            Ok(cart) => cart,

            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        };

    let nes = Nes::new(cart);

    video::run(nes);
}
