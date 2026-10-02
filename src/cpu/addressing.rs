use super::{Cpu, Mode};
use crate::bus::Bus;

impl Cpu {
    /// Remembers the address of the dummy read an indexed access makes
    /// before the high byte has been corrected: the right low byte
    /// with the *uncorrected* high byte of the base address.
    fn note_dummy_read(&mut self, base: u16, addr: u16, always: bool) {
        if self.page_crossed || always {
            self.dummy_read = Some((base & 0xff00) | (addr & 0x00ff));
        }
    }

    /// Resolves the operand address. `always_dummy` is true for stores
    /// and read-modify-write instructions: on indexed modes they always
    /// spend a cycle reading the not-yet-carried address, while plain
    /// loads only do so when the index carries into the high byte.
    pub(crate) fn operand(&mut self, bus: &mut Bus, mode: Mode, always_dummy: bool) -> Option<u16> {
        match mode {
            Mode::Imm => {
                let addr = self.pc;

                self.pc = self.pc.wrapping_add(1);

                Some(addr)
            }

            Mode::Zp => Some(self.fetch(bus) as u16),

            Mode::Zpx => Some(self.fetch(bus).wrapping_add(self.x) as u16),

            Mode::Zpy => Some(self.fetch(bus).wrapping_add(self.y) as u16),

            Mode::Abs => Some(self.word(bus)),

            Mode::Absx => {
                let base = self.word(bus);
                let addr = base.wrapping_add(self.x as u16);
                self.page_crossed = (base ^ addr) & 0xff00 != 0;
                self.note_dummy_read(base, addr, always_dummy);
                Some(addr)
            }

            Mode::Absy => {
                let base = self.word(bus);
                let addr = base.wrapping_add(self.y as u16);
                self.page_crossed = (base ^ addr) & 0xff00 != 0;
                self.note_dummy_read(base, addr, always_dummy);
                Some(addr)
            }

            Mode::Ind => {
                let pointer = self.word(bus);

                let lo = bus.read(pointer);

                let hi_address = if pointer & 0xff == 0xff {
                    pointer & 0xff00
                } else {
                    pointer + 1
                };

                let hi = bus.read(hi_address);

                Some(u16::from_le_bytes([lo, hi]))
            }

            Mode::Indx => {
                let zp = self.fetch(bus).wrapping_add(self.x);

                let lo = bus.read(zp as u16);

                let hi = bus.read(zp.wrapping_add(1) as u16);

                Some(u16::from_le_bytes([lo, hi]))
            }

            Mode::Indy => {
                let zp = self.fetch(bus);

                let lo = bus.read(zp as u16);

                let hi = bus.read(zp.wrapping_add(1) as u16);

                let base = u16::from_le_bytes([lo, hi]);
                let addr = base.wrapping_add(self.y as u16);
                self.page_crossed = (base ^ addr) & 0xff00 != 0;
                self.note_dummy_read(base, addr, always_dummy);
                Some(addr)
            }

            Mode::Rel => {
                let offset = self.fetch(bus) as i8;

                Some(self.pc.wrapping_add_signed(offset as i16))
            }

            Mode::Imp | Mode::Acc => None,
        }
    }

    pub(crate) fn load(&mut self, bus: &mut Bus, mode: Mode, addr: Option<u16>) -> u8 {
        if matches!(mode, Mode::Acc) {
            self.a
        } else {
            bus.read(addr.expect("memory operand requires address"))
        }
    }

    pub(crate) fn store(&mut self, bus: &mut Bus, mode: Mode, addr: Option<u16>, value: u8) {
        if matches!(mode, Mode::Acc) {
            self.a = value;
        } else {
            let addr = addr.expect("memory operand requires address");
            if addr >= 0x8000 {
                // Only read-modify-write instructions store through
                // here; on cartridge space they double-write.
                bus.write_rmw(addr, value);
            } else {
                bus.write(addr, value);
            }
        }
    }
}
