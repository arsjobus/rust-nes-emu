## Rust-NES-Emu

### Quick Start

`cargo run --release -- path/to/game.nes`

### Build Release

1) `cargo build --release`
2) `./target/nes roms/mario.nes`

## Project Structure

src/
├── main.rs
├── nes.rs
├── cpu/
│   ├── mod.rs
│   ├── instructions.rs
│   └── addressing.rs
├── ppu/
│   ├── mod.rs
│   └── renderer.rs
├── cartridge/
│   ├── mod.rs
│   └── mapper.rs
├── apu/
│   ├── mod.rs
│   ├── pulse.rs
│   ├── triangle.rs
│   ├── noise.rs
│   ├── dmc.rs
│   └── frame_counter.rs
├── audio.rs
├── bus.rs
├── input.rs
├── bus.rs
└── video.rs
