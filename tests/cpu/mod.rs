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

    // Taken-branch cycle penalties are not modeled yet.
    assert_eq!(cpu.step(&mut bus), 2);
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
