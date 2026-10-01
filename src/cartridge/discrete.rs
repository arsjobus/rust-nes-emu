//! Simple discrete-logic boards: one or two latches select whole PRG
//! and/or CHR banks. Bus conflicts are not modelled (the games are
//! written to avoid them), so the written value always wins.

use super::mapper::{Mapper, Mirroring};

fn prg_read(prg: &[u8], bank_size: usize, bank: usize, addr: u16) -> u8 {
    let count = (prg.len() / bank_size).max(1);
    let offset = addr as usize & (bank_size - 1);
    prg[((bank % count) * bank_size + offset) % prg.len()]
}

fn chr_index(chr: &[u8], bank_size: usize, bank: usize, addr: u16) -> usize {
    let count = (chr.len() / bank_size).max(1);
    ((bank % count) * bank_size + (addr as usize & (bank_size - 1))) % chr.len()
}

macro_rules! chr_8k_io {
    ($bank:expr) => {
        fn chr_read(&self, chr: &[u8], addr: u16) -> u8 {
            chr[chr_index(chr, 0x2000, $bank(self), addr)]
        }
        fn chr_write(&mut self, chr: &mut [u8], addr: u16, value: u8, chr_ram: bool) {
            if chr_ram {
                let index = chr_index(chr, 0x2000, $bank(self), addr);
                chr[index] = value;
            }
        }
    };
}

/// Mapper 11 - Color Dreams. $8000-$FFFF: bits 0-1 PRG (32 KiB),
/// bits 4-7 CHR (8 KiB).
pub struct ColorDreamsMapper {
    register: u8,
}

impl ColorDreamsMapper {
    pub fn new() -> Self {
        Self { register: 0 }
    }
}

impl Mapper for ColorDreamsMapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        prg_read(prg, 0x8000, (self.register & 3) as usize, addr)
    }
    fn cpu_write(&mut self, _prg: &[u8], addr: u16, value: u8) {
        if addr >= 0x8000 {
            self.register = value;
        }
    }
    chr_8k_io!(|s: &Self| (s.register >> 4) as usize);
}

/// Mapper 34 (BNROM) - $8000-$FFFF selects a 32 KiB PRG bank, CHR is RAM.
pub struct BnromMapper {
    register: u8,
}

impl BnromMapper {
    pub fn new() -> Self {
        Self { register: 0 }
    }
}

impl Mapper for BnromMapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        prg_read(prg, 0x8000, self.register as usize, addr)
    }
    fn cpu_write(&mut self, _prg: &[u8], addr: u16, value: u8) {
        if addr >= 0x8000 {
            self.register = value;
        }
    }
    fn chr_read(&self, chr: &[u8], addr: u16) -> u8 {
        chr[addr as usize % chr.len()]
    }
    fn chr_write(&mut self, chr: &mut [u8], addr: u16, value: u8, chr_ram: bool) {
        if chr_ram {
            let index = addr as usize % chr.len();
            chr[index] = value;
        }
    }
}

/// Mapper 34 (NINA-001, CHR ROM variant). Registers sit in the PRG RAM
/// window: $7FFD 32 KiB PRG, $7FFE / $7FFF two 4 KiB CHR banks.
pub struct Nina001Mapper {
    prg_bank: u8,
    chr_bank: [u8; 2],
}

impl Nina001Mapper {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            chr_bank: [0, 1],
        }
    }
    fn chr_bank_for(&self, addr: u16) -> usize {
        self.chr_bank[(addr as usize >> 12) & 1] as usize
    }
}

impl Mapper for Nina001Mapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        prg_read(prg, 0x8000, self.prg_bank as usize, addr)
    }
    fn cpu_write(&mut self, _prg: &[u8], _addr: u16, _value: u8) {}
    fn cpu_write_prg_ram_area(&mut self, addr: u16, value: u8) {
        match addr {
            0x7ffd => self.prg_bank = value,
            0x7ffe => self.chr_bank[0] = value,
            0x7fff => self.chr_bank[1] = value,
            _ => {}
        }
    }
    fn chr_read(&self, chr: &[u8], addr: u16) -> u8 {
        chr[chr_index(chr, 0x1000, self.chr_bank_for(addr), addr)]
    }
    fn chr_write(&mut self, chr: &mut [u8], addr: u16, value: u8, chr_ram: bool) {
        if chr_ram {
            let index = chr_index(chr, 0x1000, self.chr_bank_for(addr), addr);
            chr[index] = value;
        }
    }
}

/// Mapper 71 - Camerica BF9093/BF9097. $C000-$FFFF selects the 16 KiB
/// bank at $8000 (last bank fixed at $C000); $9000-$9FFF bit 4 picks
/// the one-screen page on the BF9097 (Fire Hawk).
pub struct CamericaMapper {
    bank: u8,
    mirroring: Option<Mirroring>,
}

impl CamericaMapper {
    pub fn new() -> Self {
        Self {
            bank: 0,
            mirroring: None,
        }
    }
}

impl Mapper for CamericaMapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        let count = (prg.len() / 0x4000).max(1);
        let bank = if addr < 0xc000 {
            (self.bank & 0x0f) as usize
        } else {
            count - 1
        };
        prg_read(prg, 0x4000, bank, addr)
    }
    fn cpu_write(&mut self, _prg: &[u8], addr: u16, value: u8) {
        match addr {
            0x9000..=0x9fff => {
                self.mirroring = Some(if value & 0x10 == 0 {
                    Mirroring::OneScreenLower
                } else {
                    Mirroring::OneScreenUpper
                });
            }
            0xc000..=0xffff => self.bank = value,
            _ => {}
        }
    }
    fn chr_read(&self, chr: &[u8], addr: u16) -> u8 {
        chr[addr as usize % chr.len()]
    }
    fn chr_write(&mut self, chr: &mut [u8], addr: u16, value: u8, chr_ram: bool) {
        if chr_ram {
            let index = addr as usize % chr.len();
            chr[index] = value;
        }
    }
    fn mirroring_override(&self) -> Option<Mirroring> {
        self.mirroring
    }
}

/// Mapper 79 - AVE NINA-03/NINA-06. Writes to $4100-$5FFF (with
/// address bit 8 set): bit 3 selects PRG (32 KiB), bits 0-2 CHR (8 KiB).
pub struct Nina03Mapper {
    register: u8,
}

impl Nina03Mapper {
    pub fn new() -> Self {
        Self { register: 0 }
    }
}

impl Mapper for Nina03Mapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        prg_read(prg, 0x8000, ((self.register >> 3) & 1) as usize, addr)
    }
    fn cpu_write(&mut self, _prg: &[u8], addr: u16, value: u8) {
        if (0x4100..=0x5fff).contains(&addr) && addr & 0xe100 == 0x4100 {
            self.register = value;
        }
    }
    chr_8k_io!(|s: &Self| (s.register & 7) as usize);
}

/// Mapper 87 - Jaleco/Konami discrete board. Writes to $6000-$7FFF:
/// bits 0-1 select an 8 KiB CHR bank with the two bits swapped.
pub struct Mapper87 {
    chr_bank: u8,
}

impl Mapper87 {
    pub fn new() -> Self {
        Self { chr_bank: 0 }
    }
}

impl Mapper for Mapper87 {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        prg[(addr as usize - 0x8000) % prg.len()]
    }
    fn cpu_write(&mut self, _prg: &[u8], _addr: u16, _value: u8) {}
    fn cpu_write_prg_ram_area(&mut self, _addr: u16, value: u8) {
        self.chr_bank = ((value & 1) << 1) | ((value >> 1) & 1);
    }
    chr_8k_io!(|s: &Self| s.chr_bank as usize);
}

/// Mapper 140 - Jaleco JF-11/JF-14. Writes to $6000-$7FFF: bits 4-5
/// PRG (32 KiB), bits 0-3 CHR (8 KiB).
pub struct Mapper140 {
    register: u8,
}

impl Mapper140 {
    pub fn new() -> Self {
        Self { register: 0 }
    }
}

impl Mapper for Mapper140 {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        prg_read(prg, 0x8000, ((self.register >> 4) & 3) as usize, addr)
    }
    fn cpu_write(&mut self, _prg: &[u8], _addr: u16, _value: u8) {}
    fn cpu_write_prg_ram_area(&mut self, _addr: u16, value: u8) {
        self.register = value;
    }
    chr_8k_io!(|s: &Self| (s.register & 0x0f) as usize);
}

/// Mapper 180 - UxROM with the banks swapped: first bank fixed at
/// $8000, $C000 switchable (Crazy Climber).
pub struct Uxrom180Mapper {
    bank: u8,
}

impl Uxrom180Mapper {
    pub fn new() -> Self {
        Self { bank: 0 }
    }
}

impl Mapper for Uxrom180Mapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        let bank = if addr < 0xc000 { 0 } else { self.bank as usize };
        prg_read(prg, 0x4000, bank, addr)
    }
    fn cpu_write(&mut self, _prg: &[u8], addr: u16, value: u8) {
        if addr >= 0x8000 {
            self.bank = value;
        }
    }
    fn chr_read(&self, chr: &[u8], addr: u16) -> u8 {
        chr[addr as usize % chr.len()]
    }
    fn chr_write(&mut self, chr: &mut [u8], addr: u16, value: u8, chr_ram: bool) {
        if chr_ram {
            let index = addr as usize % chr.len();
            chr[index] = value;
        }
    }
}
