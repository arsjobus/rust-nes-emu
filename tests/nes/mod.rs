use super::*;
use crate::cartridge::test_support::make_nrom_program;

/// Hand-assembled program, loaded at $8000:
///
///   8000: LDA #$5A       ; PRG RAM round trip through the CPU
///   8002: STA $6000
///   8005: LDA $6000
///   8008: STA $00
///   800A: LDA #$80       ; enable NMI
///   800C: STA $2000
///   800F: JMP $800F      ; spin
///
///   8020: INC $01        ; NMI handler counts frames
///   8022: RTI
fn program() -> Vec<u8> {
    let mut code = vec![
        0xa9, 0x5a, 0x8d, 0x00, 0x60, 0xad, 0x00, 0x60, 0x85, 0x00, 0xa9, 0x80, 0x8d, 0x00, 0x20,
        0x4c, 0x0f, 0x80,
    ];
    code.resize(0x20, 0xea);
    code.extend_from_slice(&[0xe6, 0x01, 0x40]);
    code
}

#[test]
fn runs_program_with_prg_ram_and_one_nmi_per_frame() {
    let cart = make_nrom_program(&program(), 0x8020, 0x8000, 0x8000);
    let mut nes = Nes::new(cart);

    // The first call only runs to the end of the power-on
    // pre-render line; after that each call is a full frame.
    for _ in 0..31 {
        nes.run_frame();
    }

    // Fix 1, through real CPU instructions: PRG RAM works and
    // nothing panicked reading/writing $6000.
    assert_eq!(nes.bus.ram[0], 0x5a);

    // Fix 2, end to end: exactly one NMI per vblank is
    // delivered now that `nmi_fired` is gone (30 full frames).
    let nmis = nes.bus.ram[1];
    assert!(
        (29..=30).contains(&nmis),
        "expected ~30 NMIs over 30 frames, got {nmis}"
    );
}

/// Regression: a main loop that polls $2002 for vblank while the NMI
/// handler also reads $2002 (clearing the flag) must still make
/// progress. On hardware the flag is set part-way through an
/// instruction, so a poll can see it just before the NMI is taken, and
/// RTI restores the N flag from that poll.
///
///   8000: LDA #$80 ; STA $2000       ; enable NMI
///   8005: LDA $2002 ; BPL $8005      ; wait for vblank
///   800A: INC $01                    ; one step per vblank seen
///   800C: JMP $8005
///
///   8020: LDA $2002 ; RTI            ; NMI handler clears the flag
#[test]
fn vblank_poll_loop_progresses_when_nmi_handler_reads_ppustatus() {
    let mut code = vec![
        0xa9, 0x80, 0x8d, 0x00, 0x20, 0xad, 0x02, 0x20, 0x10, 0xfb, 0xe6, 0x01, 0x4c, 0x05, 0x80,
    ];
    code.resize(0x20, 0xea);
    code.extend_from_slice(&[0xad, 0x02, 0x20, 0x40]);

    let cart = make_nrom_program(&code, 0x8020, 0x8000, 0x8000);
    let mut nes = Nes::new(cart);

    for _ in 0..30 {
        nes.run_frame();
    }

    assert!(
        nes.bus.ram[1] >= 10,
        "main loop stalled: saw vblank only {} times in 30 frames",
        nes.bus.ram[1]
    );
}
