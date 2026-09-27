## Rust-NES-Emu

<p align="center">
  <img src="cover.png" alt="Cover">
</p>

### Quick Start

`cargo run --release -- path/to/game.nes`

### Build Release

1) `cargo build --release`
2) `./target/nes roms/mario.nes`

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

--ntsc
--persistence
--bloom
--color-correction
--lut
--curvature
--auto-gradient
--scanlines
--vignette
--sprite-shadows
