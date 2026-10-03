# Bug tracker

Open issues found by the community ROM regression suite. Re-run `./scripts/run_rom_tests.sh` from the repository root after fixes. The list below reflects the run on 2026-10-03; ROM output is recorded so failures can be compared after changes.

## Open

`./scripts/run_rom_tests.sh` passes all 41 ROMs in `tests/rom-tests.tsv` as of 2026-10-03. The ROMs below, from the same collection, still fail. Status codes are the `$6000` result unless noted.

### OPEN-001: DMC DMA does not steal CPU cycles (`sprdma_and_dmc_dma`, `sprdma_and_dmc_dma_512`)

- **ROM output:** status `0x01`.
- **Cause:** DMC sample fetches are serviced instantly in `Bus::clock_apu`; the CPU is never stalled (3-4 cycles per fetch, plus the OAM DMA interaction). Needed for exact `cpu_interrupts_v2/4-irq_and_dma` as well.

### OPEN-002: Interrupt polling is instruction-granular (`cpu_interrupts_v2` 2, 3, 4, 5)

- **ROM output:** status `0x01` for each of `2-nmi_and_brk`, `3-nmi_and_irq`, `4-irq_and_dma`, `5-branch_delays_irq`.
- **Cause:** The CPU executes whole instructions. NMI hijacking of BRK/IRQ vectors, IRQ sampling on the penultimate cycle, and the branch-delay IRQ quirk need per-cycle (or at least per-poll-point) modelling.

### OPEN-003: Open bus when executing from I/O space (`cpu_exec_space`)

- **ROM output:** `test_cpu_exec_space_apu` status `0x02`; `test_cpu_exec_space_ppuio` status `0x05`.
- **Note:** the ppuio variant moved from `0x03` to `0x05` after the open-bus changes, so it may be partly fixed or may have regressed; not investigated.
- **Cause:** Not investigated beyond the status codes. Likely related to what the CPU fetches from `$2000-$401F` as opcodes/operands (open bus, `$4015` side effects).

### OPEN-004: MMC3 scanline timing (`mmc3_test/4-scanline_timing`, `mmc3_test_2/4-scanline_timing`)

- **ROM output:** status `0x02`.
- **Cause:** The renderer is scanline-based, so A12 edges are not produced on the exact dots of sprite/background fetches. `6-MMC6` and `6-MMC3_alt` fail by design: `5-MMC3` and `6-MMC3_alt` are mutually exclusive revisions, and this emulator implements the MMC3 behaviour.

### OPEN-005: Length counter write/halt timing (`blargg_apu_2005.07.30` 08, 10, 11)

- **ROM output (screen, `$01` = pass):** `08.irq_timing` `$02`, `10.len_halt_timing` `$03`, `11.len_reload_timing` `$04`. The other eight ROMs in the set show `$01`.
- **Cause:** A length-counter reload ignored when it lands on the same cycle as a length clock, a halt-flag change taking effect one cycle late, and the exact IRQ-line timing are not modelled.

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
