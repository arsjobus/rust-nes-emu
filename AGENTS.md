# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 NES emulator. The application entry point is `src/main.rs`; core machine coordination lives in `src/nes.rs` and `src/bus.rs`. Hardware is split into `src/cpu/`, `src/ppu/`, `src/apu/`, `src/cartridge/`, and `src/input/`. Video output and post-processing are in `src/video.rs` and `src/postprocess/`. The automated ROM test list and setup notes are in `tests/`; the downloaded community ROM collection lives in the locally ignored `test-roms/` directory and should not be committed.

## Build, Test, and Development Commands

- `cargo build` compiles a debug build.
- `cargo build --release` builds the optimized emulator.
- `cargo run --release -- path/to/game.nes` builds and launches a ROM.
- `cargo test` runs Rust unit and integration tests (add tests alongside modules or under `tests/` as appropriate).
- `./scripts/run_rom_tests.sh` runs the community end-to-end ROM suite. This is the primary, definitive check that the assembled emulator works correctly across CPU, PPU, and system timing; use it before considering an emulator build verified. It needs the local ROM collection described in `tests/README.md`.
- `cargo fmt --check` checks formatting; run `cargo fmt` to apply standard Rust formatting.
- `cargo clippy --all-targets` reports common Rust correctness and style issues.

## Coding Style & Naming Conventions

Use standard `rustfmt` formatting with four-space indentation. Follow Rust naming conventions: `snake_case` for modules, functions, and variables; `UpperCamelCase` for types and traits; and `SCREAMING_SNAKE_CASE` for constants. Keep hardware-specific behavior in its corresponding module, and prefer small, focused changes that preserve clear boundaries between emulation, rendering, audio, and input.

## Testing Guidelines

The ROM suite is the primary end-to-end verification: run `./scripts/run_rom_tests.sh` and treat any failing case as an unresolved compatibility issue. Its current automated subset covers ROMs that report results through the standard `$6000` status signature; see `tests/README.md` for coverage limits and ROM acquisition. Add focused unit tests for deterministic behavior such as CPU instructions, mapper reads, timing, or post-processing; name tests for the behavior they verify. Run `cargo test` for Rust tests as a supporting check. For changes affecting runtime behavior, also launch a compatible game ROM when one is available.

## Commit & Pull Request Guidelines

Recent commits use short, direct summaries, usually describing the change (for example, “Fix audio dmc”); keep commit subjects concise and action-oriented. A pull request should explain the user-visible or emulation change, note any relevant mapper or subsystem, and include the commands used to validate it. Add screenshots or recordings for visible rendering changes when useful, and do not include copyrighted ROMs.

## Configuration & Runtime Notes

Pass a `.nes` ROM path after `--` when using Cargo. Optional rendering effects are selected with command-line switches documented in `README.md`. Keep local ROM paths and machine-specific configuration out of committed source.
