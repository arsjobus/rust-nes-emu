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

        prg[index]
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
