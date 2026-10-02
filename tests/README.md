# NES test ROMs

The ROM binaries are kept locally in `test-roms/`, which is intentionally ignored by Git. They come from the community collection at <https://github.com/christopherpow/nes-test-roms> and are not copied into this project’s history.

To get the collection:

```sh
git clone --depth 1 https://github.com/christopherpow/nes-test-roms.git test-roms
```

Run the automated subset with:

```sh
./scripts/run_rom_tests.sh
```

Pass an optional substring to select test names, such as `./scripts/run_rom_tests.sh ppu_` or `./scripts/run_rom_tests.sh instr_`. The list and frame limits are in `tests/rom-tests.tsv`. The automated subset currently uses ROMs that report completion through the common `$6000` status signature. CPU instruction suites that need trace comparison, branch timing ROMs that report results by screen/beeps, and ROMs needing interactive menu selection require other harness modes and are not in this list yet.

These are third-party homebrew test ROMs. Their source, authorship, and any individual distribution terms remain with their respective authors; consult each ROM’s included documentation before redistributing the binaries.
