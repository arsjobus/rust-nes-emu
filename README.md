## Rust-NES-Emu

## Compatibility (Mapper)

Games which have mappers as:

- Mapper 0 (NROM)
- Mapper 1 (MMC1)
- Mapper 2 (UxROM)
- Mapper 3 (CNROM)
- Mapper 4 (MMC3)
- Mapper 5 (MMC5) - PRG/CHR banking, ExRAM and fill nametables, extended attributes,
  vertical split, scanline IRQs, multiplier, PRG RAM protection, and expansion audio
  (scanline features follow the emulator's scanline-level PPU timing)
- Mapper 7 [partial] (AxROM)
- Mapper 9 (MMC2)
- Mapper 66 (GxROM)

Directory: https://nesdir.github.io/

### Quick Start

`cargo run --release -- path/to/game.nes`

Start without a ROM to show animated TV static and a ROM picker:

`cargo run --release`

The idle screen lists `.nes` files in the current folder and searches `roms/` recursively. Use **Up/Down** or the controller **D-pad/left stick** to choose a game; press **Enter** or controller **A/Start** to launch it. Press the controller's **right trigger** while playing to toggle turbo for held A/B buttons. Press **O** to rescan, or **R** while playing to return to the menu. **Esc** quits.

### Build Release

1) `cargo build --release`
2) `./target/release/runes roms/{game}.nes`

SDL2 is built from source through the Rust SDL2 bindings. Building requires a C compiler and CMake; no separate SDL2 installation is needed.

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
