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

### Battery Saves

For ROMs marked as battery-backed in the iNES header, the emulator loads PRG RAM from a `.sav` file beside the ROM (for example, `game.nes` uses `game.sav`). RAM is saved when you quit, return to the ROM menu, or load another game. ROMs without the battery flag do not use save files.

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

Effects accept configurable values using either `--option value` or
`--option=value`. For example, `--bloom --bloom-strength 0.4` adjusts bloom
intensity, while `--lut cool-crt --lut-strength 0.8` selects and blends a LUT.
Available LUT presets are `identity`, `warm-crt`, `cool-crt`, `composite`,
`gameboy`, `amber`, and `high-contrast`.

Other parameters include `--bloom-threshold`, `--bloom-radius`,
`--ntsc-strength`, `--ntsc-bleed`, `--persistence-amount`,
`--persistence-frames`, `--color-brightness`, `--color-contrast`,
`--color-saturation`, `--color-gamma`, `--curvature-strength`,
`--auto-gradient-strength`, `--auto-gradient-vertical`,
`--auto-gradient-horizontal`, `--scanlines-strength`, and
`--vignette-strength`.

## Input Devies

1. Keyboard
2. Gamepad Controller (Logitech)

## Useful Links

https://www.nesdev.org/wiki/Nesdev_Wiki
