# Roadmap

This roadmap tracks practical improvements to **runes**, a Rust NES emulator. Items are proposals, not release promises; mapper support means a mapper is implemented and verified with suitable ROMs, not that every game using it is guaranteed to work.

## Near term

- [X] **Launch without a ROM:** open the emulator window with a classic untuned-TV screen (animated snow/static and subtle flicker or hum). Keep `runes <game.nes>` as the direct-to-game path, and provide a clear way to choose or open a ROM from the idle screen.
- [X] **ROM selection and library:** let users browse for a ROM, show recently played games, and explain unsupported or invalid ROM errors in the UI.
- [X] **Save RAM persistence:** load and save battery-backed cartridge RAM beside the ROM, with safe writes on exit.
- [ ] **Input configuration:** support remapping keyboard and gamepad buttons, and show connected controller status.
- [ ] **Configuration and presentation:** save preferences for display effects, scaling, audio, and controls; make effects easy to enable from the UI as well as command-line flags.

## Cartridge compatibility

The currently documented supported mapper numbers are **0 (NROM), 2 (UxROM), 9 (MMC2/PxROM), and 66 (GxROM)**. Add mapper support incrementally, with focused tests for banking, mirroring, RAM, and IRQ behavior where applicable.

- [X] **Mapper 1 — MMC1:** serial register writes, PRG/CHR banking, mirroring modes, and PRG RAM behavior.
- [X] **Mapper 4 — MMC3:** PRG/CHR banking, mirroring, RAM protection, and scanline IRQ timing.
- [X] **Mapper 3 — CNROM:** CHR bank switching and bus-conflict behavior where required by the board.
- [X] **Mapper 7 — AxROM:** PRG banking and one-screen mirroring.
- [ ] **Additional common mappers:** prioritize based on test ROM availability and requested games; candidates include 11 (Color Dreams), 23/25 (VRC), and 71 (Camerica).
- [ ] **ROM format coverage:** improve NES 2.0 header parsing and report unsupported board variants or malformed images clearly.

## Accuracy and reliability

- [ ] Expand CPU, PPU, APU, and mapper verification with established test ROMs and document results.
- [ ] Improve PPU timing and edge cases, including sprite hit/overflow, odd-frame timing, and mapper-visible PPU address activity.
- [ ] Refine CPU bus timing, interrupts, DMA, and open-bus behavior against hardware references.
- [ ] Improve APU timing and audio quality, including frame sequencing, DMC interactions, and sample-rate consistency.
- [ ] Add deterministic save states after cartridge and machine state can be serialized safely.

## User experience and platform support

- [ ] Add pause, reset, and quit controls with visible feedback.
- [ ] Add display scaling and fullscreen options, plus presets for common CRT-style effects.
- [ ] Improve startup diagnostics for missing audio devices and controller connection issues.
- [ ] Document supported platforms, controls, command-line options, and known compatibility limits.
- [ ] Package reproducible release builds for supported desktop platforms.

## Completion criteria

For each feature, update the README and add focused automated coverage where practical. For mapper work, record mapper-specific test ROMs and results, and avoid committing copyrighted ROM files. Keep gameplay, rendering, and audio changes gated on manual checks with public-domain or otherwise authorized test material when available.
