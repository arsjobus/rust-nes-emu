use std::fs;

mod mapper;

pub use mapper::{
    GxromMapper,
    Mapper,
    Mmc2Mapper,
    NromMapper,
    UxromMapper,
};

const PRG_RAM_SIZE: usize = 8 * 1024;

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

    // 8 KiB of PRG RAM mapped at $6000-$7FFF. The iNES format
    // implies this RAM exists on every board, and some ROMs (and
    // most test ROMs, which report results through $6000) expect
    // it. It is not battery backed, so it starts zeroed every run.
    pub(crate) prg_ram: [u8; PRG_RAM_SIZE],

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

        if prg_units == 0 {
            return Err(
                "ROM has no PRG data".into()
            );
        }

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
            prg_ram: [0; PRG_RAM_SIZE],
            mapper_kind,
            mirroring_vertical: vertical,
            mapper,
        })
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
            0x0000..=0x5fff => 0,

            // $6000-$7FFF: PRG RAM.
            0x6000..=0x7fff => {
                self.prg_ram[(addr - 0x6000) as usize]
            }

            // $8000-$FFFF: PRG ROM, banked by the mapper.
            0x8000..=0xffff => {
                self.mapper.cpu_read(
                    &self.prg,
                    addr,
                )
            }
        }
    }

    pub fn cpu_write(
        &mut self,
        addr: u16,
        value: u8,
    ) {
        match addr {
            // Unmapped: writes are ignored.
            0x0000..=0x5fff => {}

            // $6000-$7FFF: PRG RAM. Mapper registers on the
            // boards we support live at $8000 and above, so a
            // write here must never reach the mapper (previously
            // a stray write to this range switched GxROM banks).
            0x6000..=0x7fff => {
                self.prg_ram[(addr - 0x6000) as usize] = value;
            }

            // $8000-$FFFF: mapper registers.
            0x8000..=0xffff => {
                self.mapper.cpu_write(
                    &self.prg,
                    addr,
                    value,
                );

                // Some mappers (e.g. MMC2/MMC4) can switch
                // mirroring at runtime via a CPU-mapped register
                // rather than it being fixed by the iNES header,
                // so re-sync it after every register write.
                if let Some(vertical) =
                    self.mapper.mirroring_override()
                {
                    self.mirroring_vertical = vertical;
                }
            }
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

#[cfg(test)]
pub(crate) mod test_support {
    use super::Cartridge;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Builds a 16 KiB NROM cartridge (CHR RAM) whose PRG is the
    /// given program at $8000/$C000 plus the three vectors.
    pub(crate) fn make_nrom_program(
        code: &[u8],
        nmi: u16,
        reset: u16,
        irq: u16,
    ) -> Cartridge {
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
mod tests {
    use super::test_support::make_cart;

    /// Fix 1: reads below $8000 used to underflow and panic on
    /// NROM, UxROM and GxROM.
    #[test]
    fn low_addresses_do_not_panic_on_any_mapper() {
        let carts = [
            make_cart(0, 2, 1, false),
            make_cart(2, 8, 0, false),
            make_cart(66, 8, 4, false),
            make_cart(9, 8, 8, false),
        ];

        for cart in carts.iter() {
            for addr in [0x4020u16, 0x5fff, 0x6000, 0x7fff] {
                let _ = cart.cpu_read(addr);
            }
        }
    }

    #[test]
    fn prg_ram_round_trips_and_is_isolated_from_mappers() {
        let mut cart = make_cart(0, 2, 1, false);

        cart.cpu_write(0x6000, 0xab);
        cart.cpu_write(0x7fff, 0xcd);

        assert_eq!(cart.cpu_read(0x6000), 0xab);
        assert_eq!(cart.cpu_read(0x7fff), 0xcd);

        // Unmapped space reads as 0 and ignores writes.
        cart.cpu_write(0x5000, 0xff);
        assert_eq!(cart.cpu_read(0x5000), 0);
    }

    /// Writes to PRG RAM must not reach mapper registers.
    #[test]
    fn gxrom_ignores_writes_below_8000() {
        let mut cart = make_cart(66, 8, 4, false); // 4 x 32 KiB PRG

        assert_eq!(cart.cpu_read(0x8000), 0); // bank 0 (16K units 0,1)

        cart.cpu_write(0x6000, 0x10); // PRG RAM area: no bank change
        assert_eq!(cart.cpu_read(0x8000), 0);

        cart.cpu_write(0x8000, 0x10); // real register: PRG bank 1
        assert_eq!(cart.cpu_read(0x8000), 2);
    }

    #[test]
    fn uxrom_bank_switching_still_works() {
        let mut cart = make_cart(2, 8, 0, false);

        assert_eq!(cart.cpu_read(0xc000), 7); // fixed last bank
        cart.cpu_write(0x8000, 3);
        assert_eq!(cart.cpu_read(0x8000), 3);
        assert_eq!(cart.cpu_read(0xc000), 7);
    }

    /// MMC2 latch 0 responds to exactly $0FD8 / $0FE8 (nesdev MMC2
    /// page); it used to respond to the whole $0FD8-$0FDF range.
    #[test]
    fn mmc2_latch0_only_triggers_on_exact_addresses() {
        // 128 KiB PRG, 32 KiB CHR = eight 4 KiB banks, each 4 KiB
        // bank's bytes are its 8 KiB unit number, so use the
        // registers to pick distinguishable banks (0, 2, 4, 6).
        let mut cart = make_cart(9, 8, 4, true);

        cart.cpu_write(0xb000, 2); // latch0 = $FD -> 4KiB bank 2
        cart.cpu_write(0xc000, 4); // latch0 = $FE -> 4KiB bank 4

        let power_on = cart.chr_read(0x0000); // power-on: latch $FE
        assert_eq!(power_on, 2); // 4 KiB bank 4 lives in 8K unit 2

        // A read inside the old (wrong) range must NOT flip it.
        let _ = cart.chr_read(0x0fd9);
        assert_eq!(cart.chr_read(0x0000), 2);

        // The exact address flips to $FD (bank 2 = 8K unit 1)...
        let _ = cart.chr_read(0x0fd8);
        assert_eq!(cart.chr_read(0x0000), 1);

        // ...and $0FE8 flips it back.
        let _ = cart.chr_read(0x0fe8);
        assert_eq!(cart.chr_read(0x0000), 2);
    }

    /// Latch 1 keeps its full $1FD8-$1FDF / $1FE8-$1FEF range.
    #[test]
    fn mmc2_latch1_triggers_across_range() {
        let mut cart = make_cart(9, 8, 4, true);

        cart.cpu_write(0xd000, 2); // latch1 = $FD -> 4KiB bank 2
        cart.cpu_write(0xe000, 4); // latch1 = $FE -> 4KiB bank 4

        assert_eq!(cart.chr_read(0x1000), 2);
        let _ = cart.chr_read(0x1fdf);
        assert_eq!(cart.chr_read(0x1000), 1);
        let _ = cart.chr_read(0x1fef);
        assert_eq!(cart.chr_read(0x1000), 2);
    }

    #[test]
    fn mmc2_mirroring_register_still_syncs() {
        let mut cart = make_cart(9, 8, 4, true);
        assert!(cart.mirroring_vertical);
        cart.cpu_write(0xf000, 1); // bit 0 set = horizontal
        assert!(!cart.mirroring_vertical);
        cart.cpu_write(0xf000, 0);
        assert!(cart.mirroring_vertical);
    }
}
