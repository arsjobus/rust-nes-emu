use crate::{
    apu::Apu,
    input::Controller,
    ppu::Ppu,
};

pub struct Bus {
    pub ram: [u8; 2048],
    pub ppu: Ppu,
    pub apu: Apu,
    pub controller: Controller,
}

impl Bus {
    pub fn new(ppu: Ppu, controller: Controller) -> Self {
        Self {
            ram: [0; 2048],
            ppu,
            apu: Apu::new(),
            controller,
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1fff => {
                self.ram[(addr & 0x07ff) as usize]
            }

            0x2000..=0x3fff => {
                self.ppu.cpu_read(addr & 7)
            }

            0x4000..=0x4015 => {
                self.apu.cpu_read(addr)
            }

            0x4016 => {
                self.controller.read()
            }

            0x4017 => {
                self.apu.cpu_read(addr)
            }

            0x4018..=0x401f => 0,

            0x4020..=0xffff => {
                self.ppu.cart.cpu_read(addr)
            }
        }
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
                    self.ppu.oam[
                        (self.ppu.oam_addr as usize + i) & 255
                    ] = temp[i];
                }

                self.ppu.catch_up(513 * 3);
            }

            0x4000..=0x4015 => {
                self.apu.cpu_write(addr, value);
            }

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
