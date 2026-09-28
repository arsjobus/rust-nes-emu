pub trait Mapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8;

    fn cpu_write(
        &mut self,
        prg: &[u8],
        addr: u16,
        value: u8,
    );

    fn chr_read(
        &self,
        chr: &[u8],
        addr: u16,
    ) -> u8;

    fn chr_write(
        &mut self,
        chr: &mut [u8],
        addr: u16,
        value: u8,
        chr_ram: bool,
    );

    /// Mappers that can switch mirroring at runtime (e.g. MMC2)
    /// return their current setting here; the cartridge checks
    /// this after every CPU write and syncs it into
    /// `Cartridge::mirroring_vertical`. Mappers with fixed,
    /// header-defined mirroring just use the default.
    fn mirroring_override(&self) -> Option<bool> {
        None
    }
}

pub struct NromMapper;

impl NromMapper {
    pub fn new() -> Self {
        Self
    }
}

impl Mapper for NromMapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        let index = if prg.len() == 16 * 1024 {
            ((addr - 0x8000) & 0x3fff) as usize
        } else {
            (addr - 0x8000) as usize
        };

        prg[index]
    }

    fn cpu_write(
        &mut self,
        _prg: &[u8],
        _addr: u16,
        _value: u8,
    ) {
        // NROM has no mapper registers.
    }

    fn chr_read(
        &self,
        chr: &[u8],
        addr: u16,
    ) -> u8 {
        chr[(addr as usize) % chr.len()]
    }

    fn chr_write(
        &mut self,
        chr: &mut [u8],
        addr: u16,
        value: u8,
        chr_ram: bool,
    ) {
        if !chr_ram {
            return;
        }

        let index = (addr as usize) % chr.len();

        chr[index] = value;
    }
}


// ============================================================
// Mapper 2 - UxROM
// ============================================================
//
// CPU address space:
//
//   $8000-$BFFF -> switchable 16 KiB PRG bank
//   $C000-$FFFF -> fixed to the final 16 KiB PRG bank
//
// Writing to $8000-$FFFF selects the bank mapped at $8000.
//
// CHR is normally 8 KiB and is not bank-switched by UxROM.
// If the cartridge has CHR RAM, writes are allowed.
//

pub struct UxromMapper {
    bank_select: u8,
}

impl UxromMapper {
    pub fn new() -> Self {
        Self {
            bank_select: 0,
        }
    }
}

impl Mapper for UxromMapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        let bank_count = prg.len() / 0x4000;

        // UxROM cartridges require at least two 16 KiB PRG banks.
        assert!(
            bank_count >= 2,
            "UxROM requires at least 32 KiB of PRG ROM"
        );

        let index = if addr < 0xc000 {
            // $8000-$BFFF:
            // switchable PRG bank
            let bank = (self.bank_select as usize) % bank_count;

            bank * 0x4000 + (addr as usize - 0x8000)
        } else {
            // $C000-$FFFF:
            // fixed to the final PRG bank
            let bank = bank_count - 1;

            bank * 0x4000 + (addr as usize - 0xc000)
        };

        prg[index]
    }

    fn cpu_write(
        &mut self,
        _prg: &[u8],
        addr: u16,
        value: u8,
    ) {
        if addr >= 0x8000 {
            // UxROM uses the written value to select
            // the PRG bank mapped at $8000-$BFFF.
            //
            // The actual number of bank-select bits depends
            // on the cartridge, so the read path masks the
            // value against the available bank count.
            self.bank_select = value;
        }
    }

    fn chr_read(
        &self,
        chr: &[u8],
        addr: u16,
    ) -> u8 {
        chr[(addr as usize) % chr.len()]
    }

    fn chr_write(
        &mut self,
        chr: &mut [u8],
        addr: u16,
        value: u8,
        chr_ram: bool,
    ) {
        if !chr_ram {
            return;
        }

        let index = (addr as usize) % chr.len();

        chr[index] = value;
    }
}


// ============================================================
// Mapper 66 - GxROM
// ============================================================

pub struct GxromMapper {
    register: u8,
}

impl GxromMapper {
    pub fn new() -> Self {
        Self {
            register: 0,
        }
    }
}

impl Mapper for GxromMapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        let banks = (prg.len() / 0x8000).max(1);

        let bank =
            ((self.register >> 4) & 3) as usize % banks;

        let index =
            bank * 0x8000 +
            (addr as usize - 0x8000);

        // GxROM boards ship with >= 32 KiB of PRG, but a malformed
        // or trimmed dump must not turn into an out-of-bounds panic.
        prg[index % prg.len()]
    }

    fn cpu_write(
        &mut self,
        _prg: &[u8],
        _addr: u16,
        value: u8,
    ) {
        self.register = value;
    }

    fn chr_read(
        &self,
        chr: &[u8],
        addr: u16,
    ) -> u8 {
        let banks =
            (chr.len() / 0x2000).max(1);

        let bank =
            (self.register & 3) as usize % banks;

        chr[
            bank * 0x2000 +
            addr as usize
        ]
    }

    fn chr_write(
        &mut self,
        chr: &mut [u8],
        addr: u16,
        value: u8,
        chr_ram: bool,
    ) {
        if !chr_ram {
            return;
        }

        let banks =
            (chr.len() / 0x2000).max(1);

        let bank =
            (self.register & 3) as usize % banks;

        chr[
            bank * 0x2000 +
            addr as usize
        ] = value;
    }
}


// ============================================================
// Mapper 9 - MMC2 (PxROM)
// ============================================================
//
// Used by Punch-Out!!. The distinctive feature of MMC2 is its
// CHR "latch": each 4 KiB half of the pattern table has two
// candidate banks ($FD and $FE), and which one is currently
// mapped in is decided by the *last special tile the PPU
// fetched*, not by a CPU write.
//
// CPU address space:
//
//   $8000-$9FFF -> switchable 8 KiB PRG bank
//   $A000-$BFFF -> fixed to third-from-last 8 KiB PRG bank
//   $C000-$DFFF -> fixed to second-from-last 8 KiB PRG bank
//   $E000-$FFFF -> fixed to the last 8 KiB PRG bank
//
// CPU writes (only the low 5 bits of the value matter):
//
//   $A000-$AFFF -> PRG bank for $8000-$9FFF
//   $B000-$BFFF -> CHR bank for $0000-$0FFF, latch 0 = $FD
//   $C000-$CFFF -> CHR bank for $0000-$0FFF, latch 0 = $FE
//   $D000-$DFFF -> CHR bank for $1000-$1FFF, latch 1 = $FD
//   $E000-$EFFF -> CHR bank for $1000-$1FFF, latch 1 = $FE
//   $F000-$FFFF -> mirroring: bit 0 clear = vertical, set = horizontal
//
// PPU-side latch behavior:
//
//   Reading $0FD8 (only) sets latch 0 to $FD (selects $B000 bank).
//   Reading $0FE8 (only) sets latch 0 to $FE (selects $C000 bank).
//   Reading $1FD8-$1FDF sets latch 1 to $FD (selects $D000 bank).
//   Reading $1FE8-$1FEF sets latch 1 to $FE (selects $E000 bank).
//
// The byte at the triggering address is still served from
// whichever bank was selected *before* the flip; only later CHR
// reads see the new bank. This is why the read updates the latch
// after fetching the data rather than before.
//

use std::cell::Cell;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Latch {
    Fd,
    Fe,
}

pub struct Mmc2Mapper {
    prg_bank: u8,

    chr_bank0_fd: u8,
    chr_bank0_fe: u8,
    chr_bank1_fd: u8,
    chr_bank1_fe: u8,

    // The latches flip during CHR *reads*, which the Mapper
    // trait only hands out `&self` for, so they need interior
    // mutability rather than plain fields.
    latch0: Cell<Latch>,
    latch1: Cell<Latch>,

    mirroring_vertical: bool,
}

impl Mmc2Mapper {
    pub fn new(mirroring_vertical: bool) -> Self {
        Self {
            prg_bank: 0,
            chr_bank0_fd: 0,
            chr_bank0_fe: 0,
            chr_bank1_fd: 0,
            chr_bank1_fe: 0,
            // Real MMC2 boards power on with both latches at $FE.
            latch0: Cell::new(Latch::Fe),
            latch1: Cell::new(Latch::Fe),
            mirroring_vertical,
        }
    }
}

impl Mapper for Mmc2Mapper {
    fn cpu_read(&self, prg: &[u8], addr: u16) -> u8 {
        let bank_count = (prg.len() / 0x2000).max(1);

        let bank = match addr {
            0x8000..=0x9fff => {
                (self.prg_bank as usize) % bank_count
            }
            0xa000..=0xbfff => bank_count.saturating_sub(3),
            0xc000..=0xdfff => bank_count.saturating_sub(2),
            _ => bank_count.saturating_sub(1),
        };

        let offset = (addr as usize) & 0x1fff;

        prg[bank * 0x2000 + offset]
    }

    fn cpu_write(
        &mut self,
        _prg: &[u8],
        addr: u16,
        value: u8,
    ) {
        match addr {
            0xa000..=0xafff => {
                // PRG select is 4 bits (xxxxPPPP) on MMC2.
                self.prg_bank = value & 0x0f;
            }
            0xb000..=0xbfff => {
                self.chr_bank0_fd = value & 0x1f;
            }
            0xc000..=0xcfff => {
                self.chr_bank0_fe = value & 0x1f;
            }
            0xd000..=0xdfff => {
                self.chr_bank1_fd = value & 0x1f;
            }
            0xe000..=0xefff => {
                self.chr_bank1_fe = value & 0x1f;
            }
            0xf000..=0xffff => {
                self.mirroring_vertical = value & 1 == 0;
            }
            _ => {}
        }
    }

    fn chr_read(
        &self,
        chr: &[u8],
        addr: u16,
    ) -> u8 {
        let bank_count = (chr.len() / 0x1000).max(1);

        let (bank, half_base) = if addr < 0x1000 {
            let bank = match self.latch0.get() {
                Latch::Fd => self.chr_bank0_fd,
                Latch::Fe => self.chr_bank0_fe,
            };

            (bank, 0x0000usize)
        } else {
            let bank = match self.latch1.get() {
                Latch::Fd => self.chr_bank1_fd,
                Latch::Fe => self.chr_bank1_fe,
            };

            (bank, 0x1000usize)
        };

        let bank = (bank as usize) % bank_count;
        let offset = (addr as usize) - half_base;

        let data = chr[bank * 0x1000 + offset];

        // The latch flips *after* this byte is fetched, so it
        // only affects subsequent reads - matching real hardware.
        match addr {
            // Latch 0 responds to a single address on MMC2 (the
            // MMC4 responds to the whole $0FD8-$0FDF / $0FE8-$0FEF
            // range). Latch 1 responds to the full range on both.
            0x0fd8 => self.latch0.set(Latch::Fd),
            0x0fe8 => self.latch0.set(Latch::Fe),
            0x1fd8..=0x1fdf => self.latch1.set(Latch::Fd),
            0x1fe8..=0x1fef => self.latch1.set(Latch::Fe),
            _ => {}
        }

        data
    }

    fn chr_write(
        &mut self,
        chr: &mut [u8],
        addr: u16,
        value: u8,
        chr_ram: bool,
    ) {
        // Boards using mapper 9 always shipped with CHR ROM, but
        // guard against CHR RAM variants just in case.
        if !chr_ram {
            return;
        }

        let index = (addr as usize) % chr.len();

        chr[index] = value;
    }

    fn mirroring_override(&self) -> Option<bool> {
        Some(self.mirroring_vertical)
    }
}