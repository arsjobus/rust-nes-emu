# Bug tracker

Open issues found by the community ROM regression suite. Re-run `./scripts/run_rom_tests.sh` from the repository root after fixes. The list below reflects the run on 2026-10-02; ROM output is recorded so failures can be compared after changes.

## Open

### BUG-001: Dummy read behavior fails `instr_dummy_reads`

- **Subsystem:** CPU bus access / indexed addressing
- **ROM:** `test-roms/instr_misc/rom_singles/03-dummy_reads.nes`
- **Observed:** status `0x03`; ROM reports `LDA abs,x` and `Failed #3`.
- **Investigate:** dummy read address and cycle behavior for absolute indexed loads, especially page-crossing cases.

### BUG-002: VBL set boundary differs in `ppu_vbl_set_time`

- **Subsystem:** PPU vblank timing
- **ROM:** `test-roms/ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes`
- **Observed:** status `0x01`; first reported rows are `00 - V`, `01 - V`, `02 - V`, `03 - V`, `04 - V`, `05 V -`, `06 V -`, `07 V -`.
- **Investigate:** exact PPU-dot boundary where the vblank flag becomes visible to consecutive `$2002` reads.

### BUG-003: NMI timing is early in `ppu_nmi_timing`

- **Subsystem:** PPU NMI timing
- **ROM:** `test-roms/ppu_vbl_nmi/rom_singles/05-nmi_timing.nes`
- **Observed:** status `0x01`; ROM reports `00 3`, `01 3`, `02 3`, `03 3`, then `04 2` through `08 2`.
- **Investigate:** cycle at which NMI is delivered relative to the PPU vblank edge and CPU instruction boundary.

### BUG-004: NMI suppression window differs in `ppu_nmi_suppression`

- **Subsystem:** PPU vblank/NMI interaction
- **ROM:** `test-roms/ppu_vbl_nmi/rom_singles/06-suppression.nes`
- **Observed:** status `0x01`; row `04` reports `- N`; rows `05` and `06` report `V N`.
- **Investigate:** interaction between `$2002` reads near vblank and whether the pending NMI is suppressed.

### BUG-005: NMI enable timing fails `ppu_nmi_on_timing`

- **Subsystem:** PPU NMI timing
- **ROM:** `test-roms/ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes`
- **Observed:** status `0x01`; all reported rows `00` through `08` show NMI (`N`).
- **Investigate:** timing of enabling NMI through `$2000` around the vblank transition.

### BUG-006: NMI disable timing fails `ppu_nmi_off_timing`

- **Subsystem:** PPU NMI timing
- **ROM:** `test-roms/ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes`
- **Observed:** status `0x01`; rows `03` through `0B` show no NMI (`-`).
- **Investigate:** timing of disabling NMI through `$2000` around the vblank transition.

### BUG-007: Odd-frame clock skip behavior fails `ppu_even_odd_frames`

- **Subsystem:** PPU frame timing
- **ROM:** `test-roms/ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes`
- **Observed:** status `0x03`; ROM reports `Pattern ---BB should skip 1 clock` and `Failed #3`.
- **Investigate:** odd-frame skipped-dot behavior with background rendering enabled.

### BUG-008: Odd-frame skip occurs too early in `ppu_even_odd_timing`

- **Subsystem:** PPU frame timing
- **ROM:** `test-roms/ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes`
- **Observed:** status `0x02`; ROM reports `09` and `Clock is skipped too soon, relative to enabling BG`.
- **Investigate:** dot/scanline of the odd-frame skip relative to enabling background rendering.
