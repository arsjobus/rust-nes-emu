use std::fs;

mod mapper;

pub use mapper::{
    GxromMapper,
    Mapper,
    Mmc2Mapper,
    NromMapper,
    UxromMapper,
};

#[derive(Clone, Copy, Debug)]
pub enum MapperKind {
    Nrom,
    Uxrom,
    Gxrom,
    Mmc2,
}

pub struct Cartridge {
    pub(crate) prg: Vec<u8>,
    pub(crate) chr: Vec<u8>,

    pub(crate) chr_ram: bool,

    pub mapper_kind: MapperKind,

    pub mirroring_vertical: bool,

    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn load(path: &str) -> Result<Self, String> {
        let data =
            fs::read(path)
                .map_err(|e| e.to_string())?;

        if data.len() < 16 ||
           &data[0..4] != b"NES\x1a"
        {
            return Err(
                "Not a valid iNES ROM".into()
            );
        }

        let prg_units =
            data[4] as usize;

        let chr_units =
            data[5] as usize;

        let flags6 =
            data[6];

        let flags7 =
            data[7];

        // iNES mapper number:
        //
        // flags 7 bits 4-7 = mapper high nibble
        // flags 6 bits 4-7 = mapper low nibble
        let mapper_num =
            (flags7 & 0xf0) |
            (flags6 >> 4);

        let vertical =
            flags6 & 1 != 0;

        let trainer =
            flags6 & 4 != 0;

        let mut offset = 16;

        if trainer {
            offset += 512;
        }

        let prg_size =
            prg_units * 16_384;

        let chr_size =
            chr_units * 8_192;

        if offset + prg_size > data.len() {
            return Err(
                "ROM is truncated".into()
            );
        }

        let prg =
            data[offset..offset + prg_size]
                .to_vec();

        offset += prg_size;

        let (chr, chr_ram) =
            if chr_size == 0 {
                // No CHR ROM means the cartridge
                // uses CHR RAM.
                (
                    vec![0u8; 8192],
                    true,
                )
            } else {
                if offset + chr_size > data.len() {
                    return Err(
                        "ROM CHR data is truncated"
                            .into()
                    );
                }

                (
                    data[offset..offset + chr_size]
                        .to_vec(),
                    false,
                )
            };

        let (mapper_kind, mapper):
            (MapperKind, Box<dyn Mapper>) =
            match mapper_num {
                // Mapper 0 - NROM
                0 => (
                    MapperKind::Nrom,
                    Box::new(NromMapper::new()),
                ),

                // Mapper 2 - UxROM
                2 => (
                    MapperKind::Uxrom,
                    Box::new(UxromMapper::new()),
                ),

                // Mapper 66 - GxROM
                66 => (
                    MapperKind::Gxrom,
                    Box::new(GxromMapper::new()),
                ),

                // Mapper 9 - MMC2
                9 => (
                    MapperKind::Mmc2,
                    Box::new(Mmc2Mapper::new(vertical)),
                ),

                n => {
                    return Err(
                        format!(
                            "Unsupported mapper {}",
                            n
                        )
                    );
                }
            };

        println!("Loaded ROM: {}", path);

        println!(
            "  Mapper: {}  PRG: {}KB  CHR: {}KB  RAM={}  Mirroring={}",
            mapper_num,
            prg_size / 1024,
            chr_size / 1024,
            chr_ram,
            if vertical {
                "vertical"
            } else {
                "horizontal"
            }
        );

        Ok(Self {
            prg,
            chr,
            chr_ram,
            mapper_kind,
            mirroring_vertical: vertical,
            mapper,
        })
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        self.mapper.cpu_read(
            &self.prg,
            addr,
        )
    }

    pub fn cpu_write(
        &mut self,
        addr: u16,
        value: u8,
    ) {
        self.mapper.cpu_write(
            &self.prg,
            addr,
            value,
        );

        // Some mappers (e.g. MMC2/MMC4) can switch mirroring at
        // runtime via a CPU-mapped register rather than it being
        // fixed by the iNES header, so re-sync it after every
        // write.
        if let Some(vertical) = self.mapper.mirroring_override() {
            self.mirroring_vertical = vertical;
        }
    }

    pub fn chr_read(&self, addr: u16) -> u8 {
        self.mapper.chr_read(
            &self.chr,
            addr,
        )
    }

    pub fn chr_write(
        &mut self,
        addr: u16,
        value: u8,
    ) {
        self.mapper.chr_write(
            &mut self.chr,
            addr,
            value,
            self.chr_ram,
        );
    }
}