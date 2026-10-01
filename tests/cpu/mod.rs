use super::*;
use crate::{cartridge::test_support::make_cart, input::Controller, ppu::Ppu};

fn setup() -> (Cpu, Bus) {
    let ppu = Ppu::new(make_cart(0, 2, 0, false));
    let bus = Bus::new(ppu, Controller::new());
    (Cpu::new(), bus)
}

fn load_program(cpu: &mut Cpu, bus: &mut Bus, start: u16, bytes: &[u8]) {
    for (offset, byte) in bytes.iter().copied().enumerate() {
        bus.write(start.wrapping_add(offset as u16), byte);
    }
    cpu.pc = start;
}

#[test]
fn lda_immediate_sets_zero_and_negative_flags() {
    let (mut cpu, mut bus) = setup();
    load_program(&mut cpu, &mut bus, 0x0000, &[0xa9, 0x00, 0xa9, 0x80]);

    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.a, 0);
    assert!(cpu.flag(Z));
    assert!(!cpu.flag(N));

    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.a, 0x80);
    assert!(!cpu.flag(Z));
    assert!(cpu.flag(N));
}

#[test]
fn adc_sets_carry_and_signed_overflow() {
    let (mut cpu, mut bus) = setup();
    load_program(&mut cpu, &mut bus, 0x0000, &[0x69, 0x50, 0x69, 0x50]);
    cpu.a = 0x50;
    cpu.status &= !C;

    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.a, 0xa0);
    assert!(!cpu.flag(C));
    assert!(cpu.flag(V));
    assert!(cpu.flag(N));

    cpu.a = 0xff;
    cpu.status |= C;
    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.a, 0x50);
    assert!(cpu.flag(C));
    assert!(!cpu.flag(V));
    assert!(!cpu.flag(N));
}

#[test]
fn relative_branch_uses_signed_offset_from_next_instruction() {
    let (mut cpu, mut bus) = setup();
    load_program(&mut cpu, &mut bus, 0x0010, &[0xd0, 0xfc]);
    cpu.status &= !Z;

    // A taken branch to the same page costs one extra cycle.
    assert_eq!(cpu.step(&mut bus), 3);
    assert_eq!(cpu.pc, 0x000e);
}

#[test]
fn indirect_jmp_wraps_high_byte_within_page() {
    let (mut cpu, mut bus) = setup();
    load_program(&mut cpu, &mut bus, 0x0000, &[0x6c, 0xff, 0x02]);
    bus.write(0x02ff, 0x34);
    bus.write(0x0200, 0x12);
    bus.write(0x0300, 0x56);

    assert_eq!(cpu.step(&mut bus), 5);
    assert_eq!(cpu.pc, 0x1234);
}

#[test]
fn taken_branch_across_a_page_costs_two_extra_cycles() {
    let (mut cpu, mut bus) = setup();
    // BNE +$10 at $00F0: the next instruction is $00F2, the target
    // $0102 is on another page.
    load_program(&mut cpu, &mut bus, 0x00f0, &[0xd0, 0x10]);
    cpu.status &= !Z;

    assert_eq!(cpu.step(&mut bus), 4);
    assert_eq!(cpu.pc, 0x0102);
}

#[test]
fn branch_not_taken_costs_two_cycles() {
    let (mut cpu, mut bus) = setup();
    load_program(&mut cpu, &mut bus, 0x0010, &[0xd0, 0x10]);
    cpu.status |= Z;

    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.pc, 0x0012);
}

#[test]
fn indexed_reads_pay_a_page_cross_cycle_but_stores_do_not() {
    let (mut cpu, mut bus) = setup();
    // LDA $02FF,X ; STA $02FF,X ; LDA $0210,X
    load_program(
        &mut cpu,
        &mut bus,
        0x0000,
        &[0xbd, 0xff, 0x02, 0x9d, 0xff, 0x02, 0xbd, 0x10, 0x02],
    );
    cpu.x = 1;

    assert_eq!(cpu.step(&mut bus), 5, "LDA abs,X crossing a page");
    assert_eq!(cpu.step(&mut bus), 5, "STA abs,X is always 5");
    assert_eq!(cpu.step(&mut bus), 4, "LDA abs,X without a crossing");
}

/// Unofficial NOPs must consume their operand bytes; treating them
/// as one-byte NOPs desynchronises the instruction stream.
#[test]
fn unofficial_nops_consume_their_operands() {
    let cases: &[(&[u8], u16, u32)] = &[
        (&[0x1a], 1, 2),
        (&[0x80, 0x12], 2, 2),
        (&[0x04, 0x12], 2, 3),
        (&[0x14, 0x12], 2, 4),
        (&[0x0c, 0x34, 0x12], 3, 4),
        (&[0x1c, 0x34, 0x12], 3, 4),
    ];

    for (bytes, len, cycles) in cases {
        let (mut cpu, mut bus) = setup();
        load_program(&mut cpu, &mut bus, 0x0000, bytes);
        assert_eq!(cpu.step(&mut bus), *cycles, "{:02x?}", bytes);
        assert_eq!(cpu.pc, *len, "{:02x?}", bytes);
    }
}

#[test]
fn lax_loads_a_and_x_and_sax_stores_their_and() {
    let (mut cpu, mut bus) = setup();
    // LAX $10 ; SAX $11
    load_program(&mut cpu, &mut bus, 0x0000, &[0xa7, 0x10, 0x87, 0x11]);
    bus.write(0x0010, 0x8c);

    cpu.step(&mut bus);
    assert_eq!((cpu.a, cpu.x), (0x8c, 0x8c));
    assert!(cpu.flag(N));

    cpu.x = 0x0f;
    cpu.step(&mut bus);
    assert_eq!(bus.read(0x0011), 0x0c);
}

#[test]
fn read_modify_write_combos_match_their_official_parts() {
    let (mut cpu, mut bus) = setup();
    // DCP $10 ; ISB $11 ; SLO $12
    load_program(
        &mut cpu,
        &mut bus,
        0x0000,
        &[0xc7, 0x10, 0xe7, 0x11, 0x07, 0x12],
    );
    bus.write(0x0010, 0x41);
    bus.write(0x0011, 0x0f);
    bus.write(0x0012, 0x81);
    cpu.a = 0x40;

    cpu.step(&mut bus); // memory 0x40, compare A(0x40) with it
    assert_eq!(bus.read(0x0010), 0x40);
    assert!(cpu.flag(Z) && cpu.flag(C));

    cpu.status |= C;
    cpu.a = 0x20;
    cpu.step(&mut bus); // memory 0x10, A = 0x20 - 0x10
    assert_eq!(bus.read(0x0011), 0x10);
    assert_eq!(cpu.a, 0x10);

    cpu.step(&mut bus); // memory 0x02 with carry out, A |= 0x02
    assert_eq!(bus.read(0x0012), 0x02);
    assert!(cpu.flag(C));
    assert_eq!(cpu.a, 0x12);
}

#[test]
fn nmi_reports_seven_cycles() {
    let (mut cpu, mut bus) = setup();
    assert_eq!(cpu.nmi(&mut bus), 7);
}

/// CLI does not unmask IRQs until one more instruction has executed.
#[test]
fn irq_is_taken_one_instruction_after_cli() {
    let (mut cpu, mut bus) = setup();
    load_program(&mut cpu, &mut bus, 0x0000, &[0x58, 0xea, 0xea]); // CLI, NOP, NOP
    cpu.status |= I;

    cpu.step(&mut bus); // CLI
    assert!(!cpu.irq(&mut bus), "IRQ must not be taken right after CLI");
    cpu.step(&mut bus); // NOP
    assert!(cpu.irq(&mut bus), "IRQ is taken after the following instruction");
}

/// SEI likewise only masks IRQs after the next instruction.
#[test]
fn irq_still_fires_one_instruction_after_sei() {
    let (mut cpu, mut bus) = setup();
    load_program(&mut cpu, &mut bus, 0x0000, &[0x78, 0xea]); // SEI, NOP
    cpu.status &= !I;

    cpu.step(&mut bus); // SEI
    assert!(cpu.irq(&mut bus));
}
