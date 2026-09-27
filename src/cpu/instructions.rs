use super::{
    Cpu,
    Instruction,
    Mode,
    Op,
};

use crate::bus::Bus;

impl Cpu {
    pub(crate) fn execute(
        &mut self,
        bus: &mut Bus,
        instruction: Instruction,
        addr: Option<u16>,
    ) {
        use Op::*;

        match instruction.op {
            Adc => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.adc(value);
            }

            And => {
                self.a &=
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.zn(self.a);
            }

            Asl => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.set_flag(
                    super::C,
                    value & 0x80 != 0,
                );

                let result =
                    value << 1;

                self.store(
                    bus,
                    instruction.mode,
                    addr,
                    result,
                );

                self.zn(result);
            }

            Bcc => {
                if !self.flag(super::C) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Bcs => {
                if self.flag(super::C) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Beq => {
                if self.flag(super::Z) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Bne => {
                if !self.flag(super::Z) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Bmi => {
                if self.flag(super::N) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Bpl => {
                if !self.flag(super::N) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Bvc => {
                if !self.flag(super::V) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Bvs => {
                if self.flag(super::V) {
                    self.pc =
                        addr.unwrap();
                }
            }

            Bit => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.set_flag(
                    super::Z,
                    self.a & value == 0,
                );

                self.set_flag(
                    super::V,
                    value & 0x40 != 0,
                );

                self.set_flag(
                    super::N,
                    value & 0x80 != 0,
                );
            }

            Brk => {
                self.pc =
                    self.pc.wrapping_add(1);

                self.push(
                    bus,
                    (self.pc >> 8) as u8,
                );

                self.push(
                    bus,
                    self.pc as u8,
                );

                self.push(
                    bus,
                    self.status |
                    super::B |
                    super::U,
                );

                self.set_flag(
                    super::I,
                    true,
                );

                let lo =
                    bus.read(0xfffe);

                let hi =
                    bus.read(0xffff);

                self.pc =
                    u16::from_le_bytes([
                        lo,
                        hi,
                    ]);
            }

            Clc => {
                self.set_flag(
                    super::C,
                    false,
                );
            }

            Cld => {
                self.set_flag(
                    super::D,
                    false,
                );
            }

            Cli => {
                self.set_flag(
                    super::I,
                    false,
                );
            }

            Clv => {
                self.set_flag(
                    super::V,
                    false,
                );
            }

            Cmp => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.set_flag(
                    super::C,
                    self.a >= value,
                );

                self.zn(
                    self.a.wrapping_sub(value)
                );
            }

            Cpx => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.set_flag(
                    super::C,
                    self.x >= value,
                );

                self.zn(
                    self.x.wrapping_sub(value)
                );
            }

            Cpy => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.set_flag(
                    super::C,
                    self.y >= value,
                );

                self.zn(
                    self.y.wrapping_sub(value)
                );
            }

            Dec => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    )
                    .wrapping_sub(1);

                self.store(
                    bus,
                    instruction.mode,
                    addr,
                    value,
                );

                self.zn(value);
            }

            Dex => {
                self.x =
                    self.x.wrapping_sub(1);

                self.zn(self.x);
            }

            Dey => {
                self.y =
                    self.y.wrapping_sub(1);

                self.zn(self.y);
            }

            Eor => {
                self.a ^=
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.zn(self.a);
            }

            Inc => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    )
                    .wrapping_add(1);

                self.store(
                    bus,
                    instruction.mode,
                    addr,
                    value,
                );

                self.zn(value);
            }

            Inx => {
                self.x =
                    self.x.wrapping_add(1);

                self.zn(self.x);
            }

            Iny => {
                self.y =
                    self.y.wrapping_add(1);

                self.zn(self.y);
            }

            Jmp => {
                self.pc =
                    addr.unwrap();
            }

            Jsr => {
                let return_address =
                    self.pc.wrapping_sub(1);

                self.push(
                    bus,
                    (return_address >> 8) as u8,
                );

                self.push(
                    bus,
                    return_address as u8,
                );

                self.pc =
                    addr.unwrap();
            }

            Lda => {
                self.a =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.zn(self.a);
            }

            Ldx => {
                self.x =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.zn(self.x);
            }

            Ldy => {
                self.y =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.zn(self.y);
            }

            Lsr => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.set_flag(
                    super::C,
                    value & 1 != 0,
                );

                let result =
                    value >> 1;

                self.store(
                    bus,
                    instruction.mode,
                    addr,
                    result,
                );

                self.zn(result);
            }

            Nop => {}

            Ora => {
                self.a |=
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.zn(self.a);
            }

            Pha => {
                self.push(
                    bus,
                    self.a,
                );
            }

            Php => {
                self.push(
                    bus,
                    self.status |
                    super::B |
                    super::U,
                );
            }

            Pla => {
                self.a =
                    self.pop(bus);

                self.zn(self.a);
            }

            Plp => {
                self.status =
                    (self.pop(bus) & !super::B)
                    | super::U;
            }

            Rol => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                let carry =
                    if self.flag(super::C) {
                        1
                    } else {
                        0
                    };

                self.set_flag(
                    super::C,
                    value & 0x80 != 0,
                );

                let result =
                    (value << 1) | carry;

                self.store(
                    bus,
                    instruction.mode,
                    addr,
                    result,
                );

                self.zn(result);
            }

            Ror => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                let carry =
                    if self.flag(super::C) {
                        0x80
                    } else {
                        0
                    };

                self.set_flag(
                    super::C,
                    value & 1 != 0,
                );

                let result =
                    (value >> 1) | carry;

                self.store(
                    bus,
                    instruction.mode,
                    addr,
                    result,
                );

                self.zn(result);
            }

            Rti => {
                self.status =
                    (self.pop(bus) & !super::B)
                    | super::U;

                let lo =
                    self.pop(bus);

                let hi =
                    self.pop(bus);

                self.pc =
                    u16::from_le_bytes([
                        lo,
                        hi,
                    ]);
            }

            Rts => {
                let lo =
                    self.pop(bus);

                let hi =
                    self.pop(bus);

                self.pc =
                    u16::from_le_bytes([
                        lo,
                        hi,
                    ])
                    .wrapping_add(1);
            }

            Sbc => {
                let value =
                    self.load(
                        bus,
                        instruction.mode,
                        addr,
                    );

                self.sbc(value);
            }

            Sec => {
                self.set_flag(
                    super::C,
                    true,
                );
            }

            Sed => {
                self.set_flag(
                    super::D,
                    true,
                );
            }

            Sei => {
                self.set_flag(
                    super::I,
                    true,
                );
            }

            Sta => {
                bus.write(
                    addr.unwrap(),
                    self.a,
                );
            }

            Stx => {
                bus.write(
                    addr.unwrap(),
                    self.x,
                );
            }

            Sty => {
                bus.write(
                    addr.unwrap(),
                    self.y,
                );
            }

            Tax => {
                self.x = self.a;
                self.zn(self.x);
            }

            Tay => {
                self.y = self.a;
                self.zn(self.y);
            }

            Tsx => {
                self.x = self.sp;
                self.zn(self.x);
            }

            Txa => {
                self.a = self.x;
                self.zn(self.a);
            }

            Txs => {
                self.sp = self.x;
            }

            Tya => {
                self.a = self.y;
                self.zn(self.a);
            }
        }
    }
}

pub fn opcode_info(
    opcode: u8,
) -> Instruction {
    use Mode::*;
    use Op::*;

    match opcode {
        0x69 => Instruction::new(Adc, Imm, 2),
        0x65 => Instruction::new(Adc, Zp, 3),
        0x75 => Instruction::new(Adc, Zpx, 4),
        0x6d => Instruction::new(Adc, Abs, 4),
        0x7d => Instruction::new(Adc, Absx, 4),
        0x79 => Instruction::new(Adc, Absy, 4),
        0x61 => Instruction::new(Adc, Indx, 6),
        0x71 => Instruction::new(Adc, Indy, 5),

        0x29 => Instruction::new(And, Imm, 2),
        0x25 => Instruction::new(And, Zp, 3),
        0x35 => Instruction::new(And, Zpx, 4),
        0x2d => Instruction::new(And, Abs, 4),
        0x3d => Instruction::new(And, Absx, 4),
        0x39 => Instruction::new(And, Absy, 4),
        0x21 => Instruction::new(And, Indx, 6),
        0x31 => Instruction::new(And, Indy, 5),

        0x0a => Instruction::new(Asl, Acc, 2),
        0x06 => Instruction::new(Asl, Zp, 5),
        0x16 => Instruction::new(Asl, Zpx, 6),
        0x0e => Instruction::new(Asl, Abs, 6),
        0x1e => Instruction::new(Asl, Absx, 7),

        0x90 => Instruction::new(Bcc, Rel, 2),
        0xb0 => Instruction::new(Bcs, Rel, 2),
        0xf0 => Instruction::new(Beq, Rel, 2),
        0x30 => Instruction::new(Bmi, Rel, 2),
        0xd0 => Instruction::new(Bne, Rel, 2),
        0x10 => Instruction::new(Bpl, Rel, 2),
        0x50 => Instruction::new(Bvc, Rel, 2),
        0x70 => Instruction::new(Bvs, Rel, 2),

        0x24 => Instruction::new(Bit, Zp, 3),
        0x2c => Instruction::new(Bit, Abs, 4),

        0x00 => Instruction::new(Brk, Imp, 7),

        0x18 => Instruction::new(Clc, Imp, 2),
        0xd8 => Instruction::new(Cld, Imp, 2),
        0x58 => Instruction::new(Cli, Imp, 2),
        0xb8 => Instruction::new(Clv, Imp, 2),

        0xc9 => Instruction::new(Cmp, Imm, 2),
        0xc5 => Instruction::new(Cmp, Zp, 3),
        0xd5 => Instruction::new(Cmp, Zpx, 4),
        0xcd => Instruction::new(Cmp, Abs, 4),
        0xdd => Instruction::new(Cmp, Absx, 4),
        0xd9 => Instruction::new(Cmp, Absy, 4),
        0xc1 => Instruction::new(Cmp, Indx, 6),
        0xd1 => Instruction::new(Cmp, Indy, 5),

        0xe0 => Instruction::new(Cpx, Imm, 2),
        0xe4 => Instruction::new(Cpx, Zp, 3),
        0xec => Instruction::new(Cpx, Abs, 4),

        0xc0 => Instruction::new(Cpy, Imm, 2),
        0xc4 => Instruction::new(Cpy, Zp, 3),
        0xcc => Instruction::new(Cpy, Abs, 4),

        0xc6 => Instruction::new(Dec, Zp, 5),
        0xd6 => Instruction::new(Dec, Zpx, 6),
        0xce => Instruction::new(Dec, Abs, 6),
        0xde => Instruction::new(Dec, Absx, 7),

        0xca => Instruction::new(Dex, Imp, 2),
        0x88 => Instruction::new(Dey, Imp, 2),

        0x49 => Instruction::new(Eor, Imm, 2),
        0x45 => Instruction::new(Eor, Zp, 3),
        0x55 => Instruction::new(Eor, Zpx, 4),
        0x4d => Instruction::new(Eor, Abs, 4),
        0x5d => Instruction::new(Eor, Absx, 4),
        0x59 => Instruction::new(Eor, Absy, 4),
        0x41 => Instruction::new(Eor, Indx, 6),
        0x51 => Instruction::new(Eor, Indy, 5),

        0xe6 => Instruction::new(Inc, Zp, 5),
        0xf6 => Instruction::new(Inc, Zpx, 6),
        0xee => Instruction::new(Inc, Abs, 6),
        0xfe => Instruction::new(Inc, Absx, 7),

        0xe8 => Instruction::new(Inx, Imp, 2),
        0xc8 => Instruction::new(Iny, Imp, 2),

        0x4c => Instruction::new(Jmp, Abs, 3),
        0x6c => Instruction::new(Jmp, Ind, 5),

        0x20 => Instruction::new(Jsr, Abs, 6),

        0xa9 => Instruction::new(Lda, Imm, 2),
        0xa5 => Instruction::new(Lda, Zp, 3),
        0xb5 => Instruction::new(Lda, Zpx, 4),
        0xad => Instruction::new(Lda, Abs, 4),
        0xbd => Instruction::new(Lda, Absx, 4),
        0xb9 => Instruction::new(Lda, Absy, 4),
        0xa1 => Instruction::new(Lda, Indx, 6),
        0xb1 => Instruction::new(Lda, Indy, 5),

        0xa2 => Instruction::new(Ldx, Imm, 2),
        0xa6 => Instruction::new(Ldx, Zp, 3),
        0xb6 => Instruction::new(Ldx, Zpy, 4),
        0xae => Instruction::new(Ldx, Abs, 4),
        0xbe => Instruction::new(Ldx, Absy, 4),

        0xa0 => Instruction::new(Ldy, Imm, 2),
        0xa4 => Instruction::new(Ldy, Zp, 3),
        0xb4 => Instruction::new(Ldy, Zpx, 4),
        0xac => Instruction::new(Ldy, Abs, 4),
        0xbc => Instruction::new(Ldy, Absx, 4),

        0x4a => Instruction::new(Lsr, Acc, 2),
        0x46 => Instruction::new(Lsr, Zp, 5),
        0x56 => Instruction::new(Lsr, Zpx, 6),
        0x4e => Instruction::new(Lsr, Abs, 6),
        0x5e => Instruction::new(Lsr, Absx, 7),

        0xea => Instruction::new(Nop, Imp, 2),

        0x09 => Instruction::new(Ora, Imm, 2),
        0x05 => Instruction::new(Ora, Zp, 3),
        0x15 => Instruction::new(Ora, Zpx, 4),
        0x0d => Instruction::new(Ora, Abs, 4),
        0x1d => Instruction::new(Ora, Absx, 4),
        0x19 => Instruction::new(Ora, Absy, 4),
        0x01 => Instruction::new(Ora, Indx, 6),
        0x11 => Instruction::new(Ora, Indy, 5),

        0x48 => Instruction::new(Pha, Imp, 3),
        0x08 => Instruction::new(Php, Imp, 3),
        0x68 => Instruction::new(Pla, Imp, 4),
        0x28 => Instruction::new(Plp, Imp, 4),

        0x2a => Instruction::new(Rol, Acc, 2),
        0x26 => Instruction::new(Rol, Zp, 5),
        0x36 => Instruction::new(Rol, Zpx, 6),
        0x2e => Instruction::new(Rol, Abs, 6),
        0x3e => Instruction::new(Rol, Absx, 7),

        0x6a => Instruction::new(Ror, Acc, 2),
        0x66 => Instruction::new(Ror, Zp, 5),
        0x76 => Instruction::new(Ror, Zpx, 6),
        0x6e => Instruction::new(Ror, Abs, 6),
        0x7e => Instruction::new(Ror, Absx, 7),

        0x40 => Instruction::new(Rti, Imp, 6),
        0x60 => Instruction::new(Rts, Imp, 6),

        0xe9 => Instruction::new(Sbc, Imm, 2),
        0xe5 => Instruction::new(Sbc, Zp, 3),
        0xf5 => Instruction::new(Sbc, Zpx, 4),
        0xed => Instruction::new(Sbc, Abs, 4),
        0xfd => Instruction::new(Sbc, Absx, 4),
        0xf9 => Instruction::new(Sbc, Absy, 4),
        0xe1 => Instruction::new(Sbc, Indx, 6),
        0xf1 => Instruction::new(Sbc, Indy, 5),

        0x38 => Instruction::new(Sec, Imp, 2),
        0xf8 => Instruction::new(Sed, Imp, 2),
        0x78 => Instruction::new(Sei, Imp, 2),

        0x85 => Instruction::new(Sta, Zp, 3),
        0x95 => Instruction::new(Sta, Zpx, 4),
        0x8d => Instruction::new(Sta, Abs, 4),
        0x9d => Instruction::new(Sta, Absx, 5),
        0x99 => Instruction::new(Sta, Absy, 5),
        0x81 => Instruction::new(Sta, Indx, 6),
        0x91 => Instruction::new(Sta, Indy, 6),

        0x86 => Instruction::new(Stx, Zp, 3),
        0x96 => Instruction::new(Stx, Zpy, 4),
        0x8e => Instruction::new(Stx, Abs, 4),

        0x84 => Instruction::new(Sty, Zp, 3),
        0x94 => Instruction::new(Sty, Zpx, 4),
        0x8c => Instruction::new(Sty, Abs, 4),

        0xaa => Instruction::new(Tax, Imp, 2),
        0xa8 => Instruction::new(Tay, Imp, 2),
        0xba => Instruction::new(Tsx, Imp, 2),
        0x8a => Instruction::new(Txa, Imp, 2),
        0x9a => Instruction::new(Txs, Imp, 2),
        0x98 => Instruction::new(Tya, Imp, 2),

        _ => Instruction::new(
            Op::Nop,
            Imp,
            2,
        ),
    }
}
