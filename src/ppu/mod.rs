use crate::cartridge::Cartridge;

mod renderer;

pub(crate) const WIDTH: usize = 256;
pub(crate) const HEIGHT: usize = 240;

pub struct Ppu {
    pub(crate) cart: Cartridge,

    pub(crate) vram: [u8; 2048],
    pub(crate) palette: [u8; 32],
    pub(crate) oam: [u8; 256],

    pub(crate) ctrl: u8,
    pub(crate) mask: u8,
    pub(crate) status: u8,
    pub(crate) oam_addr: u8,

    pub(crate) v: u16,
    pub(crate) t: u16,
    pub(crate) x: u8,
    pub(crate) w: u8,

    pub(crate) read_buffer: u8,

    pub(crate) scanline: i16,
    pub(crate) dot: i32,

    pub frame_ready: bool,
    /// An NMI has been raised and is waiting for the CPU to take
    /// it (the CPU services it after the current instruction).
    pub nmi_pending: bool,

    pub(crate) sprite0_col: Option<usize>,
    pub(crate) sprite0_flagged: bool,

    pub(crate) bg_opaque: [u8; 256],

    pub framebuffer: Vec<u32>,
}

impl Ppu {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,

            vram: [0; 2048],
            palette: [0; 32],
            oam: [0; 256],

            ctrl: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,

            v: 0,
            t: 0,
            x: 0,
            w: 0,

            read_buffer: 0,

            scanline: 261,
            dot: 0,

            frame_ready: false,
            nmi_pending: false,

            sprite0_col: None,
            sprite0_flagged: true,

            bg_opaque: [0; 256],

            framebuffer: vec![0; WIDTH * HEIGHT],
        }
    }

    pub(crate) fn nt_index(&self, addr: u16) -> usize {
        let relative = addr & 0x0fff;

        let table = (relative >> 10) as usize;

        let offset = (relative & 0x03ff) as usize;

        let physical = if let Some(page) = self.cart.nametable_ciram_page(table) {
            page
        } else {
            match self.cart.mirroring {
                crate::cartridge::Mirroring::Vertical => table & 1,
                crate::cartridge::Mirroring::Horizontal => (table >> 1) & 1,
                crate::cartridge::Mirroring::OneScreenLower => 0,
                crate::cartridge::Mirroring::OneScreenUpper => 1,
            }
        };

        physical * 0x400 + offset
    }

    pub(crate) fn palette_index(addr: u16) -> usize {
        let mut index = (addr & 0x1f) as usize;

        if index >= 0x10 && index % 4 == 0 {
            index -= 0x10;
        }

        index
    }

    pub(crate) fn internal_read(&self, addr: u16) -> u8 {
        let addr = addr & 0x3fff;

        if addr < 0x2000 {
            self.cart.chr_io_read(addr)
        } else if addr < 0x3f00 {
            let rel = (addr - 0x2000) & 0x0fff;
            let attribute = rel & 0x3c0 == 0x3c0;
            let rendering = self.mask & 0x18 != 0 && (0..=239).contains(&self.scanline);
            self.cart
                .ppu_nametable_read(addr, attribute, rendering)
                .unwrap_or_else(|| self.vram[self.nt_index(addr)])
        } else {
            self.palette[Self::palette_index(addr)]
        }
    }

    pub(crate) fn internal_write(&mut self, addr: u16, value: u8) {
        let addr = addr & 0x3fff;

        if addr < 0x2000 {
            self.cart.chr_io_write(addr, value);
        } else if addr < 0x3f00 {
            let rendering = self.mask & 0x18 != 0 && (0..=239).contains(&self.scanline);
            if !self.cart.ppu_nametable_write(addr, value, rendering) {
                let index = self.nt_index(addr);
                self.vram[index] = value;
            }
        } else {
            self.palette[Self::palette_index(addr)] = value & 0x3f;
        }
    }

    pub fn cpu_read(&mut self, register: u16) -> u8 {
        match register {
            2 => {
                let result = self.status & 0xe0;

                self.status &= 0x7f;
                self.w = 0;

                result
            }

            4 => self.oam[self.oam_addr as usize],

            7 => {
                let addr = self.v & 0x3fff;

                let result;

                if addr < 0x3f00 {
                    result = self.read_buffer;

                    self.read_buffer = self.internal_read(addr);
                } else {
                    result = self.internal_read(addr);

                    self.read_buffer = self.internal_read(addr.wrapping_sub(0x1000));
                }

                self.v = self.v.wrapping_add(if self.ctrl & 4 != 0 { 32 } else { 1 }) & 0x7fff;

                result
            }

            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, register: u16, value: u8) {
        match register {
            0 => {
                let nmi_was_enabled = self.ctrl & 0x80 != 0;

                self.ctrl = value;

                let nmi_enabled = value & 0x80 != 0;

                if !nmi_enabled {
                    // Disabling NMI drops the request line, so an
                    // NMI that has been raised but not yet taken
                    // by the CPU is cancelled.
                    self.nmi_pending = false;
                } else if !nmi_was_enabled && self.status & 0x80 != 0 {
                    // Enabling NMI (0 -> 1) while the vblank flag
                    // is already set raises an NMI immediately,
                    // even mid-vblank. Games that don't read
                    // $2002 before enabling NMI depend on this.
                    self.nmi_pending = true;
                }

                self.t = (self.t & 0xf3ff) | (((value & 3) as u16) << 10);
            }

            1 => {
                self.mask = value;
            }

            3 => {
                self.oam_addr = value;
            }

            4 => {
                self.oam[self.oam_addr as usize] = value;

                self.oam_addr = self.oam_addr.wrapping_add(1);
            }

            5 => {
                if self.w == 0 {
                    self.x = value & 7;

                    self.t = (self.t & 0xffe0) | ((value >> 3) as u16);

                    self.w = 1;
                } else {
                    self.t = (self.t & 0x8fff) | (((value & 7) as u16) << 12);

                    self.t = (self.t & 0xfc1f) | (((value & 0xf8) as u16) << 2);

                    self.w = 0;
                }
            }

            6 => {
                if self.w == 0 {
                    self.t = (self.t & 0x80ff) | (((value & 0x3f) as u16) << 8);

                    self.w = 1;
                } else {
                    self.t = (self.t & 0xff00) | value as u16;

                    self.v = self.t;
                    self.w = 0;
                }
            }

            7 => {
                let addr = self.v & 0x3fff;

                self.internal_write(addr, value);

                self.v = self.v.wrapping_add(if self.ctrl & 4 != 0 { 32 } else { 1 }) & 0x7fff;
            }

            _ => {}
        }
    }

    pub(crate) fn inc_vertical(mut v: u16) -> u16 {
        if v & 0x7000 != 0x7000 {
            v += 0x1000;
        } else {
            v &= !0x7000;

            let mut y = (v & 0x03e0) >> 5;

            if y == 29 {
                y = 0;
                v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }

            v = (v & !0x03e0) | (y << 5);
        }

        v
    }

    pub fn catch_up(&mut self, mut dots: i32) {
        while dots > 0 {
            let space = 341 - self.dot;

            let step = space.min(dots);

            let old_dot = self.dot;

            self.dot += step;
            dots -= step;

            // MMC3 IRQs are clocked by the PPU address bus when
            // sprite pattern fetches raise A12, around dot 260.
            // Our scanline renderer does not model individual fetch
            // cycles, so approximate that edge at the equivalent
            // point in the scanline rather than at its beginning.
            if self.mask & 0x18 != 0
                && (0..=239).contains(&self.scanline)
                && old_dot <= 260
                && 260 < self.dot
            {
                self.cart.clock_scanline();
            }

            if self.scanline >= 0 && self.scanline < 240 && !self.sprite0_flagged {
                if let Some(column) = self.sprite0_col {
                    let threshold = column as i32 + 2;

                    if old_dot <= threshold && threshold < self.dot {
                        self.status |= 0x40;
                        self.sprite0_flagged = true;
                    }
                }
            }

            if self.dot >= 341 {
                self.dot -= 341;
                self.next_scanline();
            }
        }
    }

    fn next_scanline(&mut self) {
        self.scanline += 1;

        if self.scanline > 261 {
            self.scanline = 0;
            self.frame_ready = true;
            self.cart.ppu_frame_start();
        }

        self.cart
            .clock_mmc5_scanline(self.scanline.max(0) as u16, self.mask & 0x18 != 0);

        match self.scanline {
            0..=239 => {
                self.sprite0_col = None;
                self.sprite0_flagged = true;

                self.render_scanline(self.scanline as usize);
            }

            241 => {
                self.status |= 0x80;

                if self.ctrl & 0x80 != 0 {
                    self.nmi_pending = true;
                }
            }

            261 => {
                self.status &= !0x80;
                self.status &= !0x40;
                self.status &= !0x20;

                if self.mask & 0x18 != 0 {
                    self.v = self.t;
                }
            }

            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "../../tests/ppu/mod.rs"]
mod tests;
