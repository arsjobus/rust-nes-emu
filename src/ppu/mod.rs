use crate::cartridge::Cartridge;

mod renderer;

pub(crate) const WIDTH: usize = 256;
pub(crate) const HEIGHT: usize = 240;

pub struct Ppu {
    pub(crate) cart: Cartridge,

    pub(crate) vram: [u8; 4096],
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
    /// The pending NMI was raised by a $2000 write. Hardware takes
    /// it after the *next* instruction rather than immediately.
    pub(crate) nmi_delay: bool,
    /// Absolute dot (`cycle`) at which the pending NMI was raised. The
    /// CPU only sees an NMI that was raised before it polled for
    /// interrupts, one cycle before the end of an instruction.
    pub(crate) nmi_raised_at: u64,
    /// A $2002 read landed on the dot just before the vblank flag
    /// would have been set; the flag (and its NMI) is skipped.
    pub(crate) vbl_suppressed: bool,
    /// The frame in progress is an odd one. With background rendering
    /// enabled the pre-render line of an odd frame is one dot shorter.
    pub(crate) odd_frame: bool,

    /// Absolute PPU dot counter (monotonic), used to time A12 edges.
    pub(crate) cycle: u64,
    /// Current level of PPU address line A12 as seen by the cartridge.
    pub(crate) a12: bool,
    /// `cycle` at which A12 last went low.
    pub(crate) a12_low_since: u64,

    pub(crate) sprite0_col: Option<usize>,
    pub(crate) sprite0_flagged: bool,

    pub(crate) bg_opaque: [u8; 256],

    pub framebuffer: Vec<u32>,
}

impl Ppu {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,

            vram: [0; 4096],
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
            nmi_delay: false,
            nmi_raised_at: 0,
            vbl_suppressed: false,
            odd_frame: false,

            cycle: 0,
            a12: false,
            a12_low_since: 0,

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
                crate::cartridge::Mirroring::FourScreen => table,
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

    /// A rising edge on A12 only clocks the MMC3 counter if the line
    /// stayed low for a few CPU cycles first (the chip filters the
    /// rapid toggling that happens during sprite pattern fetches).
    const A12_FILTER_DOTS: u64 = 10;

    /// Drives the cartridge-visible A12 level. `at` is the absolute
    /// dot at which the change happens.
    pub(crate) fn a12_set(&mut self, level: bool, at: u64) {
        if level == self.a12 {
            return;
        }

        if level {
            if at.saturating_sub(self.a12_low_since) >= Self::A12_FILTER_DOTS {
                self.cart.clock_scanline();
            }
        } else {
            self.a12_low_since = at;
        }

        self.a12 = level;
    }

    /// True while the PPU is running its background/sprite fetch
    /// pipeline (and so owns the address bus).
    fn fetching(&self) -> bool {
        self.mask & 0x18 != 0 && ((0..=239).contains(&self.scanline) || self.scanline == 261)
    }

    /// The CPU put `addr` on the PPU address bus through $2006 or
    /// $2007. Only visible to the cartridge when the PPU is not
    /// fetching itself.
    fn a12_from_cpu(&mut self, addr: u16) {
        if !self.fetching() {
            let at = self.cycle;
            self.a12_set(addr & 0x1000 != 0, at);
        }
    }

    pub fn cpu_read(&mut self, register: u16) -> u8 {
        match register {
            2 => {
                match (self.scanline, self.dot) {
                    // Read on the dot just before the flag would be set:
                    // it reads clear and the flag never gets set, so
                    // no NMI is raised for this frame either.
                    (240, 340) => self.vbl_suppressed = true,
                    // Read on the dot the flag is set or the one after:
                    // it reads set but suppresses the NMI.
                    (241, 0..=1) => {
                        self.nmi_pending = false;
                        self.nmi_delay = false;
                    }
                    _ => {}
                }

                let result = self.status & 0xe0;

                self.status &= 0x7f;
                self.w = 0;

                result
            }

            4 => self.oam[self.oam_addr as usize],

            7 => {
                let addr = self.v & 0x3fff;

                self.a12_from_cpu(addr);

                let result;

                if addr < 0x3f00 {
                    result = self.read_buffer;

                    self.read_buffer = self.internal_read(addr);
                } else {
                    result = self.internal_read(addr);

                    self.read_buffer = self.internal_read(addr.wrapping_sub(0x1000));
                }

                self.v = self.v.wrapping_add(if self.ctrl & 4 != 0 { 32 } else { 1 }) & 0x7fff;
                self.a12_from_cpu(self.v);

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
                    self.nmi_delay = false;
                } else if !nmi_was_enabled
                    && self.status & 0x80 != 0
                    // The flag is cleared one dot earlier as far as the NMI
                    // enable is concerned: enabling on the last dot of
                    // vblank is already too late to raise an NMI.
                    && (self.scanline, self.dot) != (260, 340)
                {
                    self.nmi_delay = true;
                    self.nmi_raised_at = self.cycle;
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
                    self.a12_from_cpu(self.v);
                }
            }

            7 => {
                let addr = self.v & 0x3fff;

                self.a12_from_cpu(addr);

                self.internal_write(addr, value);

                self.v = self.v.wrapping_add(if self.ctrl & 4 != 0 { 32 } else { 1 }) & 0x7fff;
                self.a12_from_cpu(self.v);
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

    /// How many dots before the start of an instruction's last cycle
    /// an NMI must have been raised for the CPU's interrupt poll
    /// (taken just before the bus access of that cycle) to see it.
    pub(crate) const NMI_POLL_LEAD: u64 = 2;

    /// Whether an NMI is pending and was raised in time for a CPU
    /// interrupt poll that happens at the current dot.
    pub(crate) fn nmi_due(&self) -> bool {
        self.nmi_pending && self.nmi_raised_at + Self::NMI_POLL_LEAD <= self.cycle
    }

    /// Dot count on the pre-render line at which an odd frame decides
    /// whether to drop its last dot. Hardware decides around dot 339;
    /// this is expressed in this emulator's convention (`dot` counts
    /// completed dots, CPU accesses land on cycle boundaries), and is
    /// pinned down to exactly one value by `ppu_even_odd_timing`.
    const ODD_FRAME_SKIP_DOT: i32 = 336;

    pub fn catch_up(&mut self, mut dots: i32) {
        while dots > 0 {
            if self.scanline == 261
                && self.odd_frame
                && self.dot == Self::ODD_FRAME_SKIP_DOT
                && self.mask & 0x08 != 0
            {
                // Background rendering is on: the line ends one dot early.
                self.dot += 1;
            }

            let mut space = 341 - self.dot;

            if self.scanline == 261 && self.odd_frame && self.dot < Self::ODD_FRAME_SKIP_DOT {
                // Stop at the decision point so a later $2001 write
                // can still change the outcome.
                space = space.min(Self::ODD_FRAME_SKIP_DOT - self.dot);
            }

            let step = space.min(dots);

            let old_dot = self.dot;
            let base = self.cycle;

            self.dot += step;
            dots -= step;
            self.cycle += step as u64;

            // MMC3 IRQs are clocked by rising edges of PPU address line
            // A12. The scanline renderer does not model individual
            // fetches, so synthesize the A12 level the fetch pipeline
            // would drive at three points of every rendering line:
            //
            //   dot   5: background pattern fetches start (BG table)
            //   dot 261: sprite pattern fetches start (sprite table)
            //   dot 325: next-line background prefetch (BG table)
            //
            // With BG at $0000 / sprites at $1000 this gives the
            // classic clock at dot ~260; with BG at $1000 / sprites at
            // $0000 it gives one at ~324; with both on the same table
            // A12 never rises after the first fetch, exactly like
            // hardware. 8x16 sprites count as "high" because unused
            // sprite slots fetch tile $FF ($1000 table).
            if self.fetching() {
                let bg_high = self.ctrl & 0x10 != 0;
                let sprite_high = self.ctrl & 0x28 != 0;

                for (event_dot, level) in [(5, bg_high), (261, sprite_high), (325, bg_high)] {
                    if old_dot < event_dot && event_dot <= self.dot {
                        let at = base + (event_dot - old_dot) as u64;
                        self.a12_set(level, at);
                    }
                }
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
            self.odd_frame = !self.odd_frame;
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

            // The picture is complete once line 239 has been drawn. Signal
            // the frontend here, not at line 0 of the next frame, so the
            // presented frame never contains a line from the following one.
            240 => {
                self.frame_ready = true;
            }

            241 => {
                if self.vbl_suppressed {
                    self.vbl_suppressed = false;
                } else {
                    self.status |= 0x80;

                    if self.ctrl & 0x80 != 0 {
                        self.nmi_pending = true;
                        self.nmi_delay = false;
                        self.nmi_raised_at = self.cycle;
                    }
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
