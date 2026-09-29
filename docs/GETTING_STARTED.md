# Getting started with RuNES

RuNES is a Nintendo Entertainment System emulator written in Rust. This guide covers building it, starting a game, and using the main controls.

## What you need

- [Rust](https://www.rust-lang.org/tools/install), including Cargo.
- A C compiler and CMake. The SDL2 Rust dependency builds SDL2 from source, so a separate SDL2 installation is not normally needed.
- A compatible `.nes` ROM image. ROM images are not included with this project; provide one you are allowed to use.

## Build and launch

From the repository directory, build an optimized version:

```sh
cargo build --release
```

Run a ROM by passing its path after `--`:

```sh
cargo run --release -- path/to/game.nes
```

You can also launch the built binary directly:

```sh
./target/release/runes path/to/game.nes
```

On Windows, run `target\release\runes.exe` instead. To open the ROM picker without specifying a game, run `cargo run --release` (or launch `runes` with no arguments). The picker searches for `.nes` files in the current directory and recursively under `roms/`.

To see command line options, run:

```sh
cargo run --release -- --help
```

## Controls

| NES button | Keyboard |
| --- | --- |
| A | Z |
| B | X |
| Select | Space |
| Start | Enter |
| D-pad | Arrow keys |

USB gamepads are also supported. In the ROM picker, use Up/Down or the D-pad/left stick to select a game, then Enter or gamepad A/Start to launch it. Press **O** to rescan the ROM folders, **R** to return to the picker while playing, and **Esc** to quit. On a gamepad, the right trigger toggles turbo for held A/B buttons; **Y** enables color correction and cycles LUT presets; the left trigger starts or stops recording.

## Optional display effects

Effects are enabled with command line flags. For example:

```sh
cargo run --release -- path/to/game.nes --scanlines --ntsc --curvature
cargo run --release -- path/to/game.nes --bloom --bloom-strength 0.4 --lut cool-crt
```

Values can be supplied as `--option value` or `--option=value`. Run with `--help` for the full list of effects, tunable parameters, and defaults.

## Saves and recordings

Battery-backed games store save RAM in a `.sav` file next to the ROM. For example, `game.nes` uses `game.sav`. The emulator writes the save when you quit, return to the picker, or load another game.

Gamepad left trigger starts or stops an MP4 recording in the current working directory. Recording requires FFmpeg with `libx264` support available on `PATH`. The recording includes game audio.

## Troubleshooting

- **The ROM does not load:** Check that the file is a valid, uncompressed `.nes` image and that its mapper is supported. The current supported mapper list is in the [README](../README.md#compatibility-mapper).
- **The window or audio does not start:** Confirm your system has working video/audio support and that the SDL2 build prerequisites (a C compiler and CMake) are installed.
- **The build fails while compiling SDL2:** Install or update your platform's C build tools and CMake, then rerun `cargo build --release`.
- **Recording fails:** Install FFmpeg with `libx264` enabled and make sure `ffmpeg` can be found through `PATH`.

For emulator implementation references, see [nesdev.org](https://www.nesdev.org/wiki/Nesdev_Wiki).
