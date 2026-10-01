use crate::{apu::Apu, input::Controller, ppu::Ppu};

pub struct Bus {
    pub ram: [u8; 2048],
    pub ppu: Ppu,
    pub apu: Apu,
    pub controller: Controller,

    /// CPU cycles still owed for an OAM DMA transfer.
    dma_stall: u32,

    /// CPU cycles of the running instruction that elapse before its
    /// memory read. Set by the CPU so a PPU register read can see the
    /// PPU as it is at that cycle rather than at the start of the
    /// instruction.
    pub(crate) pre_read_cycles: u32,
    /// PPU dots already run ahead inside the current instruction.
    ppu_advanced_dots: i32,
}

impl Bus {
    pub fn new(ppu: Ppu, controller: Controller) -> Self {
        Self {
            ram: [0; 2048],
            ppu,
            apu: Apu::new(),
            controller,
            dma_stall: 0,
            pre_read_cycles: 0,
            ppu_advanced_dots: 0,
        }
    }

    /// Takes (and clears) the PPU dots already executed mid-instruction.
    pub fn take_ppu_advanced(&mut self) -> i32 {
        std::mem::take(&mut self.ppu_advanced_dots)
    }

    /// Takes (and clears) the CPU stall cycles accumulated by OAM DMA.
    pub fn take_dma_stall(&mut self) -> u32 {
        std::mem::take(&mut self.dma_stall)
    }

    pub fn clock_apu(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.apu.step_cycle();

            if let Some(addr) = self.apu.take_pending_dmc_fetch() {
                // Real hardware briefly stalls the CPU for this
                // read (DMA on the same bus); we don't model that
                // stall, just the byte transfer itself, which is
                // what actually produces correct sample audio.
                let byte = self.read(addr);
                self.apu.feed_dmc_byte(byte);
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1fff => self.ram[(addr & 0x07ff) as usize],

            0x2000..=0x3fff => {
                // Bring the PPU up to the cycle of this read first, so
                // e.g. a vblank-wait loop sees the flag at the right
                // moment instead of a whole instruction late.
                if self.pre_read_cycles > 0 {
                    let dots = (self.pre_read_cycles * 3) as i32;
                    self.pre_read_cycles = 0;
                    self.ppu.catch_up(dots);
                    self.ppu_advanced_dots += dots;
                }
                self.ppu.cpu_read(addr & 7)
            }

            0x4000..=0x4015 => self.apu.cpu_read(addr),

            0x4016 => self.controller.read(),

            0x4017 => self.apu.cpu_read(addr),

            0x4018..=0x401f => 0,

            0x5000..=0x5015 => self.apu.cpu_read(addr),

            0x4020..=0xffff => {
                let value = self.ppu.cart.cpu_read_mut(addr);
                if addr >= 0x8000 {
                    self.apu.mmc5_pcm_read(value);
                }
                value
            }
        }
    }

    /// Final write of a read-modify-write instruction to cartridge
    /// space. The real CPU writes the old value back first and then
    /// the new one on the next cycle; mappers such as MMC1 only see
    /// (and act on) the first.
    pub fn write_rmw(&mut self, addr: u16, value: u8) {
        let old = self.ppu.cart.cpu_read(addr);
        self.write(addr, old);
        self.ppu.cart.cpu_write_consecutive(addr, value);
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => {
                self.ram[(addr & 0x07ff) as usize] = value;
            }

            0x2000..=0x3fff => {
                self.ppu.cpu_write(addr & 7, value);
            }

            0x4014 => {
                let base = (value as u16) << 8;
                let mut temp = [0u8; 256];

                for i in 0..256 {
                    temp[i] = self.read(base + i as u16);
                }

                for i in 0..256 {
                    self.ppu.oam[(self.ppu.oam_addr as usize + i) & 255] = temp[i];
                }

                self.dma_stall += 513;
            }

            0x4000..=0x4015 => {
                self.apu.cpu_write(addr, value);
            }

            0x5000..=0x5015 => self.apu.cpu_write(addr, value),

            0x4016 => {
                self.controller.write(value);
            }

            0x4017 => {
                self.apu.cpu_write(addr, value);
            }

            0x4018..=0x401f => {}

            0x4020..=0xffff => {
                self.ppu.cart.cpu_write(addr, value);
            }
        }
    }
}
