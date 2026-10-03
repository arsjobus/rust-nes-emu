# Bug tracker

Open issues found by the community ROM regression suite. Re-run `./scripts/run_rom_tests.sh` from the repository root after fixes. The list below reflects the run on 2026-10-03 (updated after the ROM sweep described under BUG-020 to BUG-024, and the follow-up sweep under BUG-025 and BUG-026); ROM output is recorded so failures can be compared after changes.

## Open

`./scripts/run_rom_tests.sh` passes all 41 ROMs in `tests/rom-tests.tsv` as of 2026-10-03. The ROMs below, from the same collection, still fail. Status codes are the `$6000` result unless noted.

### OPEN-001: DMC DMA only approximates the hardware (`sprdma_and_dmc_dma`, `sprdma_and_dmc_dma_512`, `dmc_dma_during_read4`)

- **ROM output:** `sprdma_and_dmc_dma` and `_512` status `0x01` (the clock table prints 528/529 alternating; the `_512` variant 528/529 as well). `dmc_dma_during_read4`: `dma_4016_read` prints "Failed"; `dma_2007_read`, `dma_2007_write` and `double_2007_read` print the wrong values; `read_write_2007` passes.
- **Status:** each DMC fetch now stalls the CPU for 4 cycles (BUG-020), but that is the common-case average only.
- **Cause:** The halt, dummy-read and alignment cycles are not modelled individually. Real hardware (a) takes fewer extra cycles when the fetch lands inside an OAM DMA, and (b) repeats the CPU's current read during the halt cycles, which double-clocks `$2007`/`$4016` reads. An attempt to special-case "2 cycles inside an OAM DMA" changed only the `_512` numbers and was reverted because it could not be verified. Needed for exact `cpu_interrupts_v2/4-irq_and_dma` as well.

### OPEN-002: Interrupt polling is instruction-granular (`cpu_interrupts_v2` 2, 3, 4, 5)

- **ROM output:** status `0x01` for each of `2-nmi_and_brk`, `3-nmi_and_irq`, `4-irq_and_dma`, `5-branch_delays_irq`.
- **Cause:** The CPU executes whole instructions. NMI hijacking of BRK/IRQ vectors, IRQ sampling on the penultimate cycle, and the branch-delay IRQ quirk need per-cycle (or at least per-poll-point) modelling.

### OPEN-003: Open bus when executing from I/O space (`cpu_exec_space`)

- **ROM output:** `test_cpu_exec_space_apu` status `0x02`; `test_cpu_exec_space_ppuio` status `0x05`.
- **Note:** the ppuio variant moved from `0x03` to `0x05` after the open-bus changes, so it may be partly fixed or may have regressed; not investigated.
- **Cause:** Not investigated beyond the status codes. Likely related to what the CPU fetches from `$2000-$401F` as opcodes/operands (open bus, `$4015` side effects).

### OPEN-004: MMC3 scanline timing (`mmc3_test/4-scanline_timing`, `mmc3_test_2/4-scanline_timing`)

- **ROM output:** status `0x02`.
- **Cause:** The renderer is scanline-based, so A12 edges are not produced on the exact dots of sprite/background fetches. `6-MMC6` and `6-MMC3_alt` fail by design: `5-MMC3` and `6-MMC3_alt` are mutually exclusive revisions, and this emulator implements the MMC3 behaviour. The same split shows in `mmc3_irq_tests`: 1-4 and 6 pass, `5.MMC3_rev_A` fails with `#3` (rev A vs rev B IRQ behaviour), while the older `4.Scanline_timing` passes.

### OPEN-005: Reset is not reachable from the application

- `Cpu::reset`, `Nes::reset`, `Apu::reset`, `Ppu::reset` and `Bus::reset_timing` are `#[cfg(test)]`, so the shipped binary has no warm-reset path. The ROM harness uses them; the frontend does not.

### Limitations seen in the wider ROM collection (not bugs per the README's scope)

- **PAL:** `pal_apu_tests` 01-03 pass and 04-11 fail (NTSC timing only).
- **Dot-accurate PPU writes:** `scanline/scanline.nes` shows errors in every area; it needs mid-scanline `$2001/$2000/$2005/$2006` effects that a scanline renderer cannot produce.
- **Mappers:** `m22chrbankingtest`, `other/test28.nes` and `other/Streemerz_bundle.nes` stop with "Unsupported mapper 22/28".
- **`dmc_tests`** (`status`, `status_irq`, `latency`, `buffer_retained`) report only by beeps; the harness cannot score them yet, so they end in a `JMP *` loop with a blank screen and no verdict.

### Follow-up sweep (2026-10-03, second pass)

The whole `nes-test-roms` collection (263 ROMs) and `nestest` were run after BUG-025 and BUG-026. Nothing new failed beyond the OPEN items above:

- All 41 manifest ROMs and all 50 `instr_test-v3`, `instr_test-v5`, `nes_instr_test`, `instr_misc` and `instr_timing` ROMs report status `0x00`.
- Screen-reporting ROMs read from a frame dump pass: `sprite_hit_tests` (11), `vbl_nmi_timing` (7), `branch_timing_tests` (3), `cpu_timing_test6`, `blargg_nes_cpu_test5` (`official.nes` and `cpu.nes`, which needs about 2500 frames), `cpu_dummy_reads`, the `blargg_ppu_tests` (`$01`), and `mmc3_irq_tests` 1-4 and 6.
- `nestest` matches `nestest.log` for PC, A, X, Y, P, SP and cycle count on every line (run `cargo test nestest_log -- --ignored --nocapture`).

## Fixed

### BUG-001: Dummy read behavior fails `instr_dummy_reads`

- **Fix:** Indexed reads/stores now perform the dummy read at the uncorrected address (`src/cpu/addressing.rs`, `src/cpu/mod.rs`). Plain loads do it only on a page cross; stores and read-modify-write always do.

### BUG-002: VBL set boundary differs in `ppu_vbl_set_time`

- **Fix:** A `$2002` read on the dot just before vblank now suppresses the flag for that frame (`src/ppu/mod.rs`).

### BUG-003: NMI timing is early in `ppu_nmi_timing`

- **Fix:** The CPU's NMI poll now happens before the last cycle of an instruction, so an NMI raised later waits one more instruction (`src/nes.rs`, `src/ppu/mod.rs`, `src/bus.rs`).

### BUG-004: NMI suppression window differs in `ppu_nmi_suppression`

- **Fix:** A `$2002` read on the vblank-set dot or the one after returns the flag but cancels the NMI (`src/ppu/mod.rs`).

### BUG-005: NMI enable timing fails `ppu_nmi_on_timing`

- **Fix:** PPU register writes are now applied at the cycle they happen in, not at the start of the instruction, and enabling NMI on the final vblank dot is too late to raise one.

### BUG-006: NMI disable timing fails `ppu_nmi_off_timing`

- **Fix:** Same poll/write timing change as BUG-003 and BUG-005: an NMI already seen by the poll survives the `$2000` write that disables it.

### BUG-007: Odd-frame clock skip behavior fails `ppu_even_odd_frames`

- **Fix:** Odd frames drop the last pre-render dot when background rendering is on (`src/ppu/mod.rs`).

### BUG-008: Odd-frame skip occurs too early in `ppu_even_odd_timing`

- **Fix:** The odd-frame skip decision is taken at a fixed dot of the pre-render line, so it reacts to `$2001` writes at the right moment.

### BUG-009: `$4017` write took effect immediately (`apu_test` 4-jitter, 5-len_timing, 6-irq_flag_timing)

- **ROM output (before):** 4-jitter and 5-len_timing returned `0x02`; 6-irq_flag_timing returned `0x02`. 5-len_timing: "First length of mode 0 is too soon".
- **Fix:** A `$4017` write restarts the frame sequencer 3 CPU cycles later on an even cycle, 4 on an odd one; the IRQ-inhibit bit still applies at once. In 4-step mode the frame IRQ flag is now held set on cycles 29828-29830 instead of only 29829 (`src/apu/frame_counter.rs`).

### BUG-010: APU register accesses landed at the start of the instruction

- **Found by:** same ROMs as BUG-009; cycle-exact `$4015` reads were off by up to the instruction's length.
- **Fix:** `Bus::sync_apu_to_access` runs the APU up to the access cycle before any `$4000-$4017` read/write, the same approach already used for the PPU. `Nes::step_instruction` and the harness subtract the cycles already run (`src/bus.rs`, `src/nes.rs`, `src/rom_harness.rs`).

### BUG-011: Frame sequencer power-on/reset phase (`apu_reset/4017_timing`, `4017_written`)

- **ROM output (before):** `4017_timing` `0x03` ("Frame IRQ flag should be set sooner after power/reset"); `4017_written` `0x02`.
- **Fix:** The sequencer starts 8 cycles into its count after power-on and after reset (as if `$4017` had been written just before the first instruction), and a reset replays the last `$4017` value (`src/apu/frame_counter.rs`). The head start of 8 was tuned against the ROM.

### BUG-012: Reset behaved like power-on (`cpu_reset/registers`, `apu_reset/*`)

- **Fix:** Added `Cpu::power_on` (cleared registers, SP=$FD) and made `Cpu::reset` a warm reset: registers kept, SP-=3, I set. Added `Nes::reset`, `Apu::reset` (`$4015`=0, flag cleared) and `Ppu::reset`. The ROM harness now performs the reset when a test reports status `0x81`.

### BUG-013: DMC fetched its next byte a timer period late (`apu_test/8-dmc_rates`)

- **ROM output (before):** status `0x02`, "Rate 0's period is too short".
- **Fix:** The memory reader requests a byte as soon as the sample buffer is empty (on `$4015` start and the moment the output unit takes the buffer), so the sample-end IRQ lands at the right time (`src/apu/dmc.rs`).

### BUG-014: PPU I/O bus had no open-bus behaviour (`ppu_open_bus`)

- **Fix:** Added a PPU I/O latch with per-bit decay (about 0.6 s of dots; value tuned against the ROM). Write-only registers read the latch; `$2002` bits 0-4 and palette reads of `$2007` bits 6-7 come from it; `$2004` reads of a sprite's attribute byte mask the unimplemented bits 2-4 (`src/ppu/mod.rs`).

### BUG-015: Sprite overflow flag ignored the hardware bug and its timing (`sprite_overflow_tests` 1-5)

- **ROM output (before):** `1.Basics` "FAILED: #7"; all five failed.
- **Fix:** The flag now comes from `Ppu::overflow_set_dot`, which reproduces the real evaluation, including the diagonal byte-offset bug after eight sprites, and sets the flag on the dot it would happen on during the previous scanline (`src/ppu/renderer.rs`, `src/ppu/mod.rs`). Unit test added.

### BUG-016: Read-modify-write to device registers wrote only once (`cpu_dummy_writes_oam`, `cpu_dummy_writes_ppumem`)

- **ROM output (before):** status `0x05` and `0x09`.
- **Fix:** RMW instructions to `$2000-$7FFF` now write the unmodified value back before the new one (`src/cpu/addressing.rs`); previously only cartridge space double-wrote.

### BUG-017: Unstable store opcodes paid a page-cross cycle (`instr_timing/1-instr_timing`)

- **ROM output (before):** status `0x04`.
- **Fix:** `$93`, `$9B` and `$9F` are a distinct `Unstable` op with fixed 6/5/5 cycles (`src/cpu/mod.rs`, `src/cpu/instructions.rs`).

### BUG-018: OAM DMA always took 513 cycles (`oam_stress` and others)

- **Fix:** 514 cycles when the transfer starts on an odd CPU cycle (`src/bus.rs`).

### BUG-019: CPU bus open bus not modelled

- **Fix:** Reads of `$4000-$4014`, `$4018-$401F`, `$4015` bit 5 and the upper bits of `$4016/$4017` return the last value on the data bus (`src/bus.rs`).

### BUG-020: DMC sample fetches did not stall the CPU

- **Fix:** every DMC fetch adds 4 CPU cycles to the DMA stall (`Bus::DMC_DMA_STALL`, `src/bus.rs`). This is the common-case cost; see OPEN-001 for what is still approximate. No manifest ROM changed result.

### BUG-021: Halt-flag writes took effect immediately (`blargg_apu_2005.07.30/10.len_halt_timing`)

- **ROM output (before):** screen `$03` (halting on the same cycle as a length clock must be too late).
- **Fix:** pulse, triangle and noise keep a separate `length_halt` that follows the `$4000/$4004/$4008/$400C` halt bit one clock later, after the length clock of the following cycle (`apply_halt`, called from `Apu::step_cycle`). The envelope-loop flag is unaffected.

### BUG-022: Length reload on a length-clock cycle was not special-cased (`blargg_apu_2005.07.30/11.len_reload_timing`)

- **ROM output (before):** screen `$04`.
- **Fix:** a `$4003/$4007/$400B/$400F` (and MMC5 `$5003/$5007`) write that lands on the cycle before a half-frame step is ignored if the counter was non-zero; if it was zero the reload happens but that step does not clock it (`write_timed`, `FrameCounter::half_frame_next`, `src/apu/`).

### BUG-023: Frame IRQ was taken one cycle too early (`blargg_apu_2005.07.30/08.irq_timing`)

- **ROM output (before):** screen `$02` (handler entered 29832 cycles after the `$4017` write; hardware needs at least 29833).
- **Fix:** the CPU's interrupt poll now samples the APU IRQ line before the instruction's last cycle and sees it one cycle late (`Apu::irq_line_polled`, `src/nes.rs`). Before, the line was read after the whole instruction had run.

### BUG-024: Palette RAM started zeroed (`blargg_ppu_tests_2005.09.15b/power_up_palette`)

- **ROM output (before):** screen `$02`.
- **Fix:** palette RAM starts with the commonly documented power-on table (`Ppu::POWER_UP_PALETTE`, `src/ppu/mod.rs`). Real consoles vary; the ROM's own readme says its table is probably unique to the author's unit.

### BUG-025: Green flash at start-up (regression from BUG-024)

- **Symptom:** the first frames of most games (for example `roms/pong.nes`, frames 0 and 1 were `0x083a00`) showed dark green before the game's own picture appeared.
- **Cause:** BUG-024 started palette RAM at the power-on table, whose entry 0 (`$09`) is a dark green. Games keep rendering off during boot, so the renderer drew that backdrop until the game wrote `$3F00`.
- **Fix:** `Ppu::backdrop_written` is set by the first write to `$3F00`/`$3F10`; until then the picture shows black (`$0F`). Palette RAM itself is unchanged, so `power_up_palette` still passes (`src/ppu/mod.rs`, `src/ppu/renderer.rs`). Unit test added.

### BUG-026: AHX, TAS, LAS and XAA were stubs (found by code review; formerly OPEN-005)

- **Fix:** `$93`/`$9F` (AHX) store `A & X & (H+1)`, `$9B` (TAS) sets `SP = A & X` and stores `SP & (H+1)`, `$BB` (LAS) sets `A = X = SP = mem & SP`, and `$8B` (XAA) sets `A = X & imm` (magic constant `$FF`). The stores replace the target's high byte on a page cross, like SHX/SHY. Cycle counts are unchanged (BUG-017), LAS pays the usual page-cross cycle (`src/cpu/mod.rs`, `src/cpu/instructions.rs`). Unit tests added for each. Real hardware results of XAA and the AHX/TAS masking vary between consoles, so the common-emulator behaviour is used.
- **Tooling:** added the ignored `nestest_log` test (`src/rom_harness.rs`) that compares every `nestest.log` line.
