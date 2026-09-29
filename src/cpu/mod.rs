use crate::bus::Bus;

mod addressing;
mod instructions;

use instructions::opcode_info;

pub const C: u8 = 0x01;
pub const Z: u8 = 0x02;
pub const I: u8 = 0x04;
pub const D: u8 = 0x08;
pub const B: u8 = 0x10;
pub const U: u8 = 0x20;
pub const V: u8 = 0x40;
pub const N: u8 = 0x80;

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Imp,
    Acc,
    Imm,
    Zp,
    Zpx,
    Zpy,
    Abs,
    Absx,
    Absy,
    Ind,
    Indx,
    Indy,
    Rel,
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Adc,
    And,
    Asl,

    Bcc,
    Bcs,
    Beq,
    Bmi,
    Bne,
    Bpl,
    Bvc,
    Bvs,

    Bit,
    Brk,

    Clc,
    Cld,
    Cli,
    Clv,

    Cmp,
    Cpx,
    Cpy,

    Dec,
    Dex,
    Dey,

    Eor,

    Inc,
    Inx,
    Iny,

    Jmp,
    Jsr,

    Lda,
    Ldx,
    Ldy,

    Lsr,

    Nop,

    Ora,

    Pha,
    Php,
    Pla,
    Plp,

    Rol,
    Ror,

    Rti,
    Rts,

    Sbc,

    Sec,
    Sed,
    Sei,

    Sta,
    Stx,
    Sty,

    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
}

#[derive(Clone, Copy)]
pub struct Instruction {
    pub op: Op,
    pub mode: Mode,
    pub cycles: u32,
}

impl Instruction {
    pub const fn new(op: Op, mode: Mode, cycles: u32) -> Self {
        Self { op, mode, cycles }
    }
}

pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,

    pub sp: u8,
    pub pc: u16,

    pub status: u8,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,

            sp: 0xfd,
            pc: 0,

            status: 0x24,
        }
    }

    pub fn reset(&mut self, bus: &mut Bus) {
        let lo = bus.read(0xfffc);

        let hi = bus.read(0xfffd);

        self.pc = u16::from_le_bytes([lo, hi]);

        self.sp = 0xfd;
        self.status = 0x24;

        self.a = 0;
        self.x = 0;
        self.y = 0;
    }

    pub fn step(&mut self, bus: &mut Bus) -> u32 {
        let opcode = self.fetch(bus);

        let instruction = opcode_info(opcode);

        let addr = if matches!(instruction.mode, Mode::Imp | Mode::Acc) {
            None
        } else {
            self.operand(bus, instruction.mode)
        };

        self.execute(bus, instruction, addr);

        instruction.cycles
    }

    pub fn nmi(&mut self, bus: &mut Bus) {
        self.push(bus, (self.pc >> 8) as u8);

        self.push(bus, self.pc as u8);

        self.push(bus, (self.status & !B) | U);

        self.set_flag(I, true);

        let lo = bus.read(0xfffa);

        let hi = bus.read(0xfffb);

        self.pc = u16::from_le_bytes([lo, hi]);
    }

    /*
     * Maskable interrupt (IRQ). Level-triggered: the caller should
     * invoke this only when an IRQ source is asserted, and it is a
     * no-op while the interrupt-disable flag is set. Pushes PC and
     * status (B clear), sets I, and jumps through the $FFFE vector.
     * Returns true if the interrupt was actually taken.
     */
    pub fn irq(&mut self, bus: &mut Bus) -> bool {
        if self.flag(I) {
            return false;
        }

        self.push(bus, (self.pc >> 8) as u8);

        self.push(bus, self.pc as u8);

        self.push(bus, (self.status & !B) | U);

        self.set_flag(I, true);

        let lo = bus.read(0xfffe);

        let hi = bus.read(0xffff);

        self.pc = u16::from_le_bytes([lo, hi]);

        true
    }

    pub(crate) fn flag(&self, flag: u8) -> bool {
        self.status & flag != 0
    }

    pub(crate) fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }

    pub(crate) fn zn(&mut self, value: u8) {
        self.set_flag(Z, value == 0);

        self.set_flag(N, value & 0x80 != 0);
    }

    pub(crate) fn push(&mut self, bus: &mut Bus, value: u8) {
        bus.write(0x0100 + self.sp as u16, value);

        self.sp = self.sp.wrapping_sub(1);
    }

    pub(crate) fn pop(&mut self, bus: &mut Bus) -> u8 {
        self.sp = self.sp.wrapping_add(1);

        bus.read(0x0100 + self.sp as u16)
    }

    pub(crate) fn fetch(&mut self, bus: &mut Bus) -> u8 {
        let value = bus.read(self.pc);

        self.pc = self.pc.wrapping_add(1);

        value
    }

    pub(crate) fn word(&mut self, bus: &mut Bus) -> u16 {
        let lo = self.fetch(bus);

        let hi = self.fetch(bus);

        u16::from_le_bytes([lo, hi])
    }

    fn adc(&mut self, value: u8) {
        let a = self.a;

        let carry = if self.flag(C) { 1u16 } else { 0 };

        let result = a as u16 + value as u16 + carry;

        let result8 = result as u8;

        self.set_flag(C, result > 0xff);

        self.set_flag(V, (!(a ^ value) & (a ^ result8) & 0x80) != 0);

        self.a = result8;

        self.zn(result8);
    }

    fn sbc(&mut self, value: u8) {
        self.adc(value ^ 0xff);
    }
}

#[cfg(test)]
#[path = "../../tests/cpu/mod.rs"]
mod tests;
