use std::{fs, path::PathBuf};

mod discrete;
mod mapper;

use discrete::{
    BnromMapper, CamericaMapper, ColorDreamsMapper, Mapper140, Mapper87, Nina001Mapper,
    Nina03Mapper, Uxrom180Mapper,
};

pub use mapper::{
    AxromMapper, CnromMapper, GxromMapper, Mapper, Mirroring, Mmc1Mapper, Mmc2Mapper, Mmc3Mapper,
    Mmc5Mapper, NromMapper, UxromMapper,
};

const PRG_RAM_SIZE: usize = 64 * 1024;

#[derive(Clone, Copy, Debug)]
pub enum MapperKind {
    Nrom,
    Uxrom,
    Cnrom,
    Gxrom,
    Axrom,
    Mmc2,
    Mmc1,
    Mmc3,
    Mmc5,
    ColorDreams,
    Bnrom,
    Nina001,
    Camerica,
    Nina03,
    Jaleco87,
    Jaleco140,
    Uxrom180,
    Namco108,
    TxSrom,
}

pub struct Cartridge {
    pub(crate) prg: Vec<u8>,
    pub(crate) chr: Vec<u8>,

    pub(crate) chr_ram: bool,

    // Storage for up to 64 KiB of PRG RAM. Non-MMC5 boards use the
    // first 8 KiB; MMC5 selects 8 KiB banks from the larger region.
    // Battery-backed boards persist the first `save_size` bytes.
    pub(crate) prg_ram: Vec<u8>,
    battery_backed: bool,
    /// Number of bytes of `prg_ram` written to / read from the `.sav` file.
    save_size: usize,
    save_path: Option<PathBuf>,
    four_screen: bool,

    #[allow(dead_code)] // Exposed cartridge metadata for callers that inspect loaded ROMs.
    pub mapper_kind: MapperKind,

    pub mirroring_vertical: bool,
    pub(crate) mirroring: Mirroring,

    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn load(path: &str) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| e.to_string())?;

        if data.len() < 16 || &data[0..4] != b"NES\x1a" {
            return Err("Not a valid iNES ROM".into());
        }

        let mut prg_units = data[4] as usize;

        let mut chr_units = data[5] as usize;

        let flags6 = data[6];

        let flags7 = data[7];

        // iNES mapper number:
        //
        // flags 7 bits 4-7 = mapper high nibble
        // flags 6 bits 4-7 = mapper low nibble
        //
        // Old dumps tagged by tools such as "DiskDude!" leave text
        // in header bytes 7-15, which corrupts flags 7 and yields a
        // bogus mapper number. iNES 1.0 headers must have bytes
        // 12-15 zero, so when they aren't (and the header isn't
        // NES 2.0) the high nibble in flags 7 is ignored.
        let nes2 = flags7 & 0x0c == 0x08;
        let dirty_header = !nes2 && data[12..16].iter().any(|&b| b != 0);
        let mut mapper_num: u16 = if dirty_header {
            (flags6 >> 4) as u16
        } else {
            ((flags7 & 0xf0) | (flags6 >> 4)) as u16
        };

        // NES 2.0 extends the mapper number to 12 bits and the ROM
        // sizes (non-exponent form) to 12 bits each.
        if nes2 {
            mapper_num |= ((data[8] & 0x0f) as u16) << 8;
            // 0xF in a size nibble selects exponent-multiplier notation,
            // which this loader does not implement; reading it as a
            // plain unit count would silently map the wrong data.
            if data[9] & 0x0f == 0x0f || data[9] >> 4 == 0x0f {
                return Err("NES 2.0 exponent-notation ROM sizes are not supported".into());
            }
            prg_units |= ((data[9] & 0x0f) as usize) << 8;
            chr_units |= ((data[9] >> 4) as usize) << 8;
        }

        let vertical = flags6 & 1 != 0;
        let four_screen = flags6 & 8 != 0;

        let trainer = flags6 & 4 != 0;
        let battery_backed = flags6 & 2 != 0;

        if prg_units == 0 {
            return Err("ROM has no PRG data".into());
        }

        let mut offset = 16;

        if trainer {
            offset += 512;
        }

        let prg_size = prg_units * 16_384;

        let chr_size = chr_units * 8_192;

        if offset + prg_size > data.len() {
            return Err("ROM is truncated".into());
        }

        let prg = data[offset..offset + prg_size].to_vec();

        let save_path = battery_backed.then(|| {
            let mut path = std::path::Path::new(path).to_path_buf();
            path.set_extension("sav");
            path
        });

        offset += prg_size;

        let (chr, chr_ram) = if chr_size == 0 {
            // No CHR ROM means the cartridge uses CHR RAM. NES 2.0
            // headers state its size (64 << n bytes); otherwise 8 KiB.
            let ram_shift = if nes2 { data[11] & 0x0f } else { 0 };
            let ram_size = if ram_shift > 0 {
                (64usize << ram_shift).max(8192)
            } else {
                8192
            };
            (vec![0u8; ram_size], true)
        } else {
            if offset + chr_size > data.len() {
                return Err("ROM CHR data is truncated".into());
            }

            (data[offset..offset + chr_size].to_vec(), false)
        };

        let (mapper_kind, mapper): (MapperKind, Box<dyn Mapper>) = match mapper_num {
            // Mapper 0 - NROM
            0 => (MapperKind::Nrom, Box::new(NromMapper::new())),

            // Mapper 2 - UxROM
            2 => (MapperKind::Uxrom, Box::new(UxromMapper::new())),

            // Mapper 3 - CNROM
            3 => (MapperKind::Cnrom, Box::new(CnromMapper::new())),

            // Mapper 66 - GxROM
            66 => (MapperKind::Gxrom, Box::new(GxromMapper::new())),

            // Mapper 7 - AxROM
            7 => (MapperKind::Axrom, Box::new(AxromMapper::new())),

            // Mapper 9 - MMC2
            9 => (MapperKind::Mmc2, Box::new(Mmc2Mapper::new(vertical))),
            1 => (MapperKind::Mmc1, Box::new(Mmc1Mapper::new(vertical))),
            4 => (MapperKind::Mmc3, Box::new(Mmc3Mapper::new(vertical))),
            5 => (MapperKind::Mmc5, Box::new(Mmc5Mapper::new(vertical))),

            // Discrete-logic boards and MMC3 relatives.
            11 => (MapperKind::ColorDreams, Box::new(ColorDreamsMapper::new())),
            34 if chr_ram => (MapperKind::Bnrom, Box::new(BnromMapper::new())),
            34 => (MapperKind::Nina001, Box::new(Nina001Mapper::new())),
            71 => (MapperKind::Camerica, Box::new(CamericaMapper::new())),
            79 => (MapperKind::Nina03, Box::new(Nina03Mapper::new())),
            87 => (MapperKind::Jaleco87, Box::new(Mapper87::new())),
            118 => (MapperKind::TxSrom, Box::new(Mmc3Mapper::txsrom(vertical))),
            140 => (MapperKind::Jaleco140, Box::new(Mapper140::new())),
            180 => (MapperKind::Uxrom180, Box::new(Uxrom180Mapper::new())),
            206 => (MapperKind::Namco108, Box::new(Mmc3Mapper::namco108(vertical))),

            n => {
                return Err(format!("Unsupported mapper {}", n));
            }
        };

        println!("Loaded ROM: {}", path);

        println!(
            "  Mapper: {}  PRG: {}KB  CHR: {}KB  RAM={}  Mirroring={}",
            mapper_num,
            prg_size / 1024,
            chr_size / 1024,
            chr_ram,
            if four_screen {
                "four-screen"
            } else if vertical {
                "vertical"
            } else {
                "horizontal"
            }
        );

        let mirroring = if four_screen {
            Mirroring::FourScreen
        } else {
            mapper.mirroring_override().unwrap_or(if vertical {
                Mirroring::Vertical
            } else {
                Mirroring::Horizontal
            })
        };

        // Size of the battery file. MMC5 boards vary and the header is
        // often wrong for them, so keep the full window there.
        let save_size = if matches!(mapper_kind, MapperKind::Mmc5) {
            PRG_RAM_SIZE
        } else if nes2 {
            match data[10] >> 4 {
                0 => 8192,
                shift => 64usize << shift,
            }
        } else if dirty_header {
            8192
        } else {
            (data[8] as usize).max(1) * 8192
        }
        .clamp(8192, PRG_RAM_SIZE);

        let mut cartridge = Self {
            prg,
            chr,
            chr_ram,
            prg_ram: vec![0; PRG_RAM_SIZE],
            battery_backed,
            save_size,
            save_path,
            four_screen,
            mapper_kind,
            mirroring_vertical: mirroring == Mirroring::Vertical,
            mirroring,
            mapper,
        };
        if trainer {
            // The 512-byte trainer is mapped at $7000-$71FF.
            cartridge.prg_ram[0x1000..0x1200].copy_from_slice(&data[16..16 + 512]);
        }
        if let Some(path) = &cartridge.save_path {
            match fs::read(path) {
                Ok(saved) => {
                    let count = saved.len().min(cartridge.save_size);
                    cartridge.prg_ram[..count].copy_from_slice(&saved[..count]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => eprintln!("Could not load save {}: {}", path.display(), error),
            }
        }
        Ok(cartridge)
    }

    /// Atomically persist battery-backed PRG RAM beside the ROM.
    pub fn save_battery_ram(&self) -> Result<(), String> {
        if !self.battery_backed {
            return Ok(());
        }
        let path = self.save_path.as_ref().ok_or("Missing save path")?;
        let mut temporary = path.as_os_str().to_os_string();
        temporary.push(format!(".{}.tmp", std::process::id()));
        let temporary = PathBuf::from(temporary);
        let result = (|| {
            use std::io::Write;
            let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
            file.write_all(&self.prg_ram[..self.save_size])
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temporary, path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// CPU-side cartridge read. The bus forwards all of
    /// $4020-$FFFF here, so this is the one place that decides
    /// which part of that range is PRG RAM, mapper-controlled ROM,
    /// or nothing at all. The individual mappers only ever see
    /// addresses in $8000-$FFFF.
    pub fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            // $4020-$5FFF: unmapped on all supported boards.
            // (Real hardware returns open bus; 0 is close enough.)
            0x0000..=0x5fff => self.mapper.cpu_peek_ext(addr).unwrap_or(0),

            // $6000-$7FFF: PRG RAM.
            0x6000..=0x7fff => {
                if self.mapper.prg_ram_enabled() {
                    self.prg_ram[self.mapper.cpu_ram_index(addr) % self.prg_ram.len()]
                } else {
                    0
                }
            }

            // $8000-$FFFF: PRG ROM, banked by the mapper.
            0x8000..=0xffff => self
                .mapper
                .read_slot_ram_index(addr)
                .map(|i| self.prg_ram[i % self.prg_ram.len()])
                .unwrap_or_else(|| self.mapper.cpu_read(&self.prg, addr)),
        }
    }

    pub(crate) fn cpu_read_mut(&mut self, addr: u16) -> u8 {
        if addr <= 0x5fff {
            self.mapper.cpu_read_ext(addr).unwrap_or(0)
        } else {
            self.cpu_read(addr)
        }
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        match addr {
            // $4020-$5FFF: only expansion-register mappers (MMC5 at
            // $5000+, NINA-03 at $4100+) decode this range. Everything
            // below $4100 is unmapped and must never reach a mapper,
            // or a stray write would clock e.g. the MMC1 shift
            // register. Mappers ignore addresses they don't decode.
            0x0000..=0x40ff => {}
            0x4100..=0x5fff => {
                self.mapper.cpu_write(&self.prg, addr, value);
            }

            // $6000-$7FFF: PRG RAM. Mapper registers on the
            // boards we support live at $8000 and above, so a
            // write here must never reach the mapper (previously
            // a stray write to this range switched GxROM banks).
            0x6000..=0x7fff => {
                self.mapper.cpu_write_prg_ram_area(addr, value);
                if self.mapper.prg_ram_writable() {
                    let index = self.mapper.cpu_ram_index(addr) % self.prg_ram.len();
                    self.prg_ram[index] = value;
                }
            }

            // $8000-$FFFF: mapper registers.
            0x8000..=0xffff => {
                let ram_slot = self.mapper.prg_slot_is_ram(addr);
                if ram_slot {
                    self.cpu_write_prg_slot(addr, value);
                } else {
                    self.mapper.cpu_write(&self.prg, addr, value);
                }

                // Some mappers (e.g. MMC2/MMC4) can switch
                // mirroring at runtime via a CPU-mapped register
                // rather than it being fixed by the iNES header,
                // so re-sync it after every register write.
                // (Four-screen boards have fixed mirroring.)
                if let (false, Some(mirroring)) =
                    (self.four_screen, self.mapper.mirroring_override())
                {
                    self.mirroring_vertical = mirroring == Mirroring::Vertical;
                    self.mirroring = mirroring;
                }
            }
        }
    }

    /// Second write of a read-modify-write instruction (the CPU has
    /// already sent the unmodified value as the first write).
    pub(crate) fn cpu_write_consecutive(&mut self, addr: u16, value: u8) {
        if addr >= 0x8000 && self.mapper.ignores_consecutive_writes() {
            return;
        }
        self.cpu_write(addr, value);
    }

    pub(crate) fn cpu_write_prg_slot(&mut self, addr: u16, value: u8) -> bool {
        if let Some(index) = self.mapper.write_slot_ram_index(addr) {
            let index = index % self.prg_ram.len();
            self.prg_ram[index] = value;
            true
        } else {
            false
        }
    }
    pub(crate) fn ppu_nametable_read(
        &self,
        addr: u16,
        attribute: bool,
        rendering: bool,
    ) -> Option<u8> {
        self.mapper.ppu_nametable_read(addr, attribute, rendering)
    }
    pub(crate) fn ppu_nametable_write(&mut self, addr: u16, value: u8, rendering: bool) -> bool {
        self.mapper.ppu_nametable_write(addr, value, rendering)
    }
    pub(crate) fn nametable_ciram_page(&self, table: usize) -> Option<usize> {
        self.mapper.nametable_ciram_page(table)
    }
    pub(crate) fn clock_mmc5_scanline(&mut self, scanline: u16, rendering: bool) {
        self.mapper.clock_mmc5_scanline(scanline, rendering);
    }
    pub(crate) fn ppu_frame_start(&mut self) {
        self.mapper.ppu_frame_start();
    }
    pub(crate) fn bg_tile(&self, x: usize, y: usize) -> Option<(u8, u8, u16)> {
        self.mapper.background_tile(x, y)
    }
    pub(crate) fn bg_chr_read(
        &self,
        addr: u16,
        x: usize,
        y: usize,
        separate_bg_regs: bool,
        nt: (usize, usize),
    ) -> u8 {
        self.mapper
            .background_chr_read(&self.chr, addr, x, y, separate_bg_regs, nt)
            .unwrap_or_else(|| self.mapper.chr_read(&self.chr, addr))
    }
    pub(crate) fn bg_palette(&self, x: usize, y: usize, nt: (usize, usize)) -> Option<u8> {
        self.mapper.ppu_palette_for_split(x, y, nt)
    }

    #[allow(dead_code)] // Direct cartridge access is used by mapper-level tests.
    pub fn chr_read(&self, addr: u16) -> u8 {
        self.mapper.chr_read(&self.chr, addr)
    }

    pub(crate) fn chr_io_read(&self, addr: u16) -> u8 {
        self.mapper.chr_io_read(&self.chr, addr)
    }
    pub(crate) fn sprite_chr_read(&self, addr: u16) -> u8 {
        self.mapper.chr_read(&self.chr, addr)
    }

    #[allow(dead_code)] // Direct cartridge access is used by mapper-level tests.
    pub fn chr_write(&mut self, addr: u16, value: u8) {
        self.mapper
            .chr_write(&mut self.chr, addr, value, self.chr_ram);
    }

    pub(crate) fn chr_io_write(&mut self, addr: u16, value: u8) {
        self.mapper
            .chr_io_write(&mut self.chr, addr, value, self.chr_ram);
    }

    pub(crate) fn clock_scanline(&mut self) {
        self.mapper.clock_scanline();
    }
    pub(crate) fn irq_pending(&self) -> bool {
        self.mapper.irq_pending()
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::Cartridge;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Builds a 16 KiB NROM cartridge (CHR RAM) whose PRG is the
    /// given program at $8000/$C000 plus the three vectors.
    pub(crate) fn make_nrom_program(code: &[u8], nmi: u16, reset: u16, irq: u16) -> Cartridge {
        let mut prg = vec![0xeau8; 0x4000]; // NOP-filled
        prg[..code.len()].copy_from_slice(code);
        prg[0x3ffa..0x3ffc].copy_from_slice(&nmi.to_le_bytes());
        prg[0x3ffc..0x3ffe].copy_from_slice(&reset.to_le_bytes());
        prg[0x3ffe..0x4000].copy_from_slice(&irq.to_le_bytes());

        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(b"NES\x1a");
        data[4] = 1; // one 16 KiB PRG bank, no CHR -> CHR RAM
        data.extend_from_slice(&prg);

        let path = std::env::temp_dir().join(format!(
            "runes_prog_{}_{}.nes",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst),
        ));

        std::fs::write(&path, &data).unwrap();
        let cart = Cartridge::load(path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        cart
    }

    pub(crate) fn unique_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "runes_img_{}_{}.nes",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst),
        ))
    }

    /// Loads raw iNES bytes through the real `Cartridge::load` path.
    pub(crate) fn load_image(data: &[u8]) -> Result<Cartridge, String> {
        let path = unique_path();
        std::fs::write(&path, data).unwrap();
        let cart = Cartridge::load(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sav"));
        cart
    }

    /// A 16-byte iNES header followed by `prg_units` x 16 KiB of zeroes and
    /// `chr_units` x 8 KiB of zeroes (plus a trainer when flags 6 bit 2 is set).
    pub(crate) fn raw_ines(prg_units: u8, chr_units: u8, flags6: u8, flags7: u8) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(b"NES\x1a");
        data[4] = prg_units;
        data[5] = chr_units;
        data[6] = flags6;
        data[7] = flags7;
        if flags6 & 4 != 0 {
            data.extend((0..512).map(|i| (i % 251) as u8 + 1));
        }
        data.extend(std::iter::repeat(0).take(prg_units as usize * 0x4000));
        data.extend(std::iter::repeat(0).take(chr_units as usize * 0x2000));
        data
    }

    /// Builds a throwaway iNES image and loads it through the real
    /// `Cartridge::load` path. Every 16 KiB PRG bank is filled with
    /// its own bank number and every 8 KiB CHR unit with its unit
    /// number, so a read tells you which bank is mapped.
    pub(crate) fn make_cart(
        mapper: u8,
        prg_16k_units: u8,
        chr_8k_units: u8,
        vertical: bool,
    ) -> Cartridge {
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(b"NES\x1a");
        data[4] = prg_16k_units;
        data[5] = chr_8k_units;
        data[6] = (mapper << 4) | (vertical as u8);
        data[7] = mapper & 0xf0;

        for bank in 0..prg_16k_units {
            data.extend(std::iter::repeat(bank).take(0x4000));
        }
        for unit in 0..chr_8k_units {
            data.extend(std::iter::repeat(unit).take(0x2000));
        }

        let path = std::env::temp_dir().join(format!(
            "runes_test_{}_{}.nes",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst),
        ));

        std::fs::write(&path, &data).unwrap();
        let cart = Cartridge::load(path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        cart
    }
}

#[cfg(test)]
#[path = "../../tests/cartridge/mod.rs"]
mod tests;
