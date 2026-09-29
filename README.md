## Rust-NES-Emu

## Compatibility (Mapper)

Games which have mappers as:

- Mapper 0
- Mapper 2
- Mapper 9
- Mapper 66

Directory: https://nesdir.github.io/

### Quick Start

`cargo run --release -- path/to/game.nes`

### Build Release

1) `cargo build --release`
2) `./target/release/runes roms/{game}.nes`

## Project Structure

```
src/
├── main.rs
├── nes.rs
├── cpu/
│   ├── mod.rs
│   ├── instructions.rs
│   └── addressing.rs
├── postprocess/
│   ├── auto_gradient.rs
│   ├── bloom.rs
│   ├── color_correction.rs
│   ├── curvature.rs
│   ├── effect.rs
│   ├── lut.rs
│   ├── mod.rs
│   ├── ntcs.rs
│   ├── persistence.rs
│   ├── pipeline.rs
│   ├── scanlines.rs
│   ├── vignette.rs
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
```

## Post Processing Effect Switches

1. --ntsc
2. --persistence
3. --bloom
4. --color-correction
5. --lut
6. --curvature
7. --auto-gradient
8. --scanlines
9. --vignette

## Input Devies

1. Keyboard
2. Gamepad Controller (Logitech)

## Useful Links

https://www.nesdev.org/wiki/Nesdev_Wiki
