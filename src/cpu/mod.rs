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

    // Unofficial ("illegal") opcodes that commercial games and
    // test ROMs rely on.
    Lax,
    Sax,
    Dcp,
    Isb,
    Slo,
    Rla,
    Sre,
    Rra,
    Anc,
    Alr,
    Arr,
    Axs,
    Shx,
    Shy,
}

impl Op {
    /// True for read-type instructions that take an extra cycle when
    /// an indexed effective address crosses a page boundary.
    pub(crate) fn has_page_cross_penalty(self) -> bool {
        matches!(
            self,
            Op::Adc
                | Op::And
                | Op::Cmp
                | Op::Eor
                | Op::Lda
                | Op::Ldx
                | Op::Ldy
                | Op::Ora
                | Op::Sbc
                | Op::Nop
                | Op::Lax
        )
    }
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

    /// Extra cycles accrued by the instruction currently executing
    /// (taken branches, page crossings).
    pub(crate) extra_cycles: u32,
    /// Set by the addressing-mode code when an indexed access
    /// crossed a page boundary.
    pub(crate) page_crossed: bool,
    /// Address of the dummy read the current indexed access performs
    /// while the CPU fixes up the high byte of the address.
    pub(crate) dummy_read: Option<u16>,
    /// The I flag as the interrupt poll saw it after the last
    /// instruction. CLI, SEI and PLP change I too late for the poll
    /// at the end of their own execution, so an IRQ is judged against
    /// the *old* value and takes effect one instruction later.
    irq_poll_i: Option<bool>,
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

            extra_cycles: 0,
            page_crossed: false,
            dummy_read: None,
            irq_poll_i: None,
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
        self.extra_cycles = 0;
        self.page_crossed = false;
        self.dummy_read = None;

        let opcode = self.fetch(bus);

        let instruction = opcode_info(opcode);

        let addr = if matches!(instruction.mode, Mode::Imp | Mode::Acc) {
            None
        } else {
            self.operand(
                bus,
                instruction.mode,
                !instruction.op.has_page_cross_penalty(),
            )
        };

        // Read-type instructions pay one extra cycle when an
        // indexed address crosses a page. Stores and
        // read-modify-write instructions always take the long
        // path and already include it in their base cycle count.
        if self.page_crossed && instruction.op.has_page_cross_penalty() {
            self.extra_cycles += 1;
        }

        let i_before = self.flag(I);

        let total_cycles = instruction.cycles + self.extra_cycles;

        // Indexed accesses read the uncorrected address one cycle before
        // the real access. Memory-mapped registers such as $2002 see it.
        if let Some(dummy) = self.dummy_read.take() {
            bus.set_access_cycle(total_cycles.saturating_sub(2), false);
            bus.read(dummy);
        }

        // The instruction's real memory access happens on its last
        // cycle; the interrupt poll happens at the end of the one before.
        bus.set_access_cycle(total_cycles.saturating_sub(1), true);

        self.execute(bus, instruction, addr);

        bus.set_access_cycle(0, false);

        self.irq_poll_i = matches!(instruction.op, Op::Cli | Op::Sei | Op::Plp).then_some(i_before);

        instruction.cycles + self.extra_cycles
    }

    /// Serviced NMI. Returns the 7 cycles the sequence takes so the
    /// caller can keep the PPU/APU in step.
    pub fn nmi(&mut self, bus: &mut Bus) -> u32 {
        self.push(bus, (self.pc >> 8) as u8);

        self.push(bus, self.pc as u8);

        self.push(bus, (self.status & !B) | U);

        self.set_flag(I, true);

        let lo = bus.read(0xfffa);

        let hi = bus.read(0xfffb);

        self.pc = u16::from_le_bytes([lo, hi]);

        7
    }

    /*
     * Maskable interrupt (IRQ). Level-triggered: the caller should
     * invoke this only when an IRQ source is asserted, and it is a
     * no-op while the interrupt-disable flag is set. Pushes PC and
     * status (B clear), sets I, and jumps through the $FFFE vector.
     * Returns true if the interrupt was actually taken.
     */
    pub fn irq(&mut self, bus: &mut Bus) -> bool {
        let masked = self.irq_poll_i.take().unwrap_or_else(|| self.flag(I));

        if masked {
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
