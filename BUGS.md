# Bug tracker

Open issues found by the community ROM regression suite. Re-run `./scripts/run_rom_tests.sh` from the repository root after fixes. The list below reflects the run on 2026-10-02; ROM output is recorded so failures can be compared after changes.

## Open

None. `./scripts/run_rom_tests.sh` passes all 13 ROMs as of 2026-10-02.

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
