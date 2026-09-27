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
