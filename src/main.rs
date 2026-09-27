use minifb::{Key, Window, WindowOptions};
use std::{env, fs, time::{Duration, Instant}};

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

const NES_PALETTE: [(u8,u8,u8);64] = [
    (84,84,84),(0,30,116),(8,16,144),(48,0,136),(68,0,100),(92,0,48),(84,4,0),(60,24,0),
    (32,42,0),(8,58,0),(0,64,0),(0,60,0),(0,50,60),(0,0,0),(0,0,0),(0,0,0),
    (152,150,152),(8,76,196),(48,50,236),(92,30,228),(136,20,176),(160,20,100),(152,34,32),(120,60,0),
    (84,90,0),(40,114,0),(8,124,0),(0,118,40),(0,102,120),(0,0,0),(0,0,0),(0,0,0),
    (236,238,236),(76,154,236),(120,124,236),(176,98,236),(228,84,236),(236,88,180),(236,106,100),(212,136,32),
    (160,170,0),(116,196,0),(76,208,32),(56,204,108),(56,180,204),(60,60,60),(0,0,0),(0,0,0),
    (236,238,236),(168,204,236),(188,188,236),(212,178,236),(236,174,236),(236,174,212),(236,180,176),(228,196,144),
    (204,210,120),(180,222,120),(168,226,144),(152,226,180),(160,214,228),(160,162,160),(0,0,0),(0,0,0),
];

#[derive(Clone, Copy)]
enum MapperKind {
    Nrom,
    Gxrom,
}

struct Cartridge {
    prg: Vec<u8>,
    chr: Vec<u8>,
    chr_ram: bool,
    mapper: MapperKind,
    mirroring_vertical: bool,
    mapper_reg: u8,
}

impl Cartridge {
    fn load(path: &str) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| e.to_string())?;

        if data.len() < 16 || &data[0..4] != b"NES\x1a" {
            return Err("Not a valid iNES ROM".into());
        }

        let prg_units = data[4] as usize;
        let chr_units = data[5] as usize;
        let flags6 = data[6];
        let flags7 = data[7];

        let mapper_num = (flags7 & 0xf0) | (flags6 >> 4);
        let vertical = flags6 & 1 != 0;
        let trainer = flags6 & 4 != 0;

        let mut off = 16;

        if trainer {
            off += 512;
        }

        let prg_size = prg_units * 16384;
        let chr_size = chr_units * 8192;

        if off + prg_size > data.len() {
            return Err("ROM is truncated".into());
        }

        let prg = data[off..off + prg_size].to_vec();
        off += prg_size;

        let (chr, chr_ram) = if chr_size == 0 {
            (vec![0u8; 8192], true)
        } else {
            if off + chr_size > data.len() {
                return Err("ROM CHR data is truncated".into());
            }

            (data[off..off + chr_size].to_vec(), false)
        };

        let mapper = match mapper_num {
            0 => MapperKind::Nrom,
            66 => MapperKind::Gxrom,
            n => return Err(format!("Unsupported mapper {}", n)),
        };

        println!("Loaded ROM: {}", path);

        println!(
            "  Mapper: {}  PRG: {}KB  CHR: {}KB  RAM={}  Mirroring={}",
            mapper_num,
            prg_size / 1024,
            chr_size / 1024,
            chr_ram,
            if vertical { "vertical" } else { "horizontal" }
        );

        Ok(Self {
            prg,
            chr,
            chr_ram,
            mapper,
            mirroring_vertical: vertical,
            mapper_reg: 0,
        })
    }

    fn cpu_read(&self, addr: u16) -> u8 {
        match self.mapper {
            MapperKind::Nrom => {
                let i = if self.prg.len() == 16384 {
                    ((addr - 0x8000) & 0x3fff) as usize
                } else {
                    (addr - 0x8000) as usize
                };

                self.prg[i]
            }

            MapperKind::Gxrom => {
                let banks = (self.prg.len() / 0x8000).max(1);
                let bank = ((self.mapper_reg >> 4) & 3) as usize % banks;

                self.prg[
                    bank * 0x8000 +
                    (addr as usize - 0x8000)
                ]
            }
        }
    }

    fn cpu_write(&mut self, _addr: u16, value: u8) {
        if matches!(self.mapper, MapperKind::Gxrom) {
            self.mapper_reg = value;
        }
    }

    fn chr_read(&self, addr: u16) -> u8 {
        match self.mapper {
            MapperKind::Nrom => {
                self.chr[(addr as usize) % self.chr.len()]
            }

            MapperKind::Gxrom => {
                let banks = (self.chr.len() / 0x2000).max(1);
                let bank = (self.mapper_reg & 3) as usize % banks;

                self.chr[
                    bank * 0x2000 +
                    addr as usize
                ]
            }
        }
    }

    fn chr_write(&mut self, addr: u16, value: u8) {
        if !self.chr_ram {
            return;
        }

        match self.mapper {
            MapperKind::Nrom => {
                let i = addr as usize % self.chr.len();
                self.chr[i] = value;
            }

            MapperKind::Gxrom => {
                let banks = (self.chr.len() / 0x2000).max(1);
                let bank = (self.mapper_reg & 3) as usize % banks;

                self.chr[
                    bank * 0x2000 +
                    addr as usize
                ] = value;
            }
        }
    }
}

struct Controller {
    state: u8,
    index: u8,
    strobe: bool,
}

impl Controller {
    fn new() -> Self {
        Self {
            state: 0,
            index: 0,
            strobe: false,
        }
    }

    fn update(&mut self, window: &Window) {
        let mut s = 0;

        if window.is_key_down(Key::Z) {
            s |= 1 << 0;
        }

        if window.is_key_down(Key::X) {
            s |= 1 << 1;
        }

        if window.is_key_down(Key::RightShift) {
            s |= 1 << 2;
        }

        if window.is_key_down(Key::Enter) {
            s |= 1 << 3;
        }

        if window.is_key_down(Key::Up) {
            s |= 1 << 4;
        }

        if window.is_key_down(Key::Down) {
            s |= 1 << 5;
        }

        if window.is_key_down(Key::Left) {
            s |= 1 << 6;
        }

        if window.is_key_down(Key::Right) {
            s |= 1 << 7;
        }

        self.state = s;
    }

    fn write(&mut self, value: u8) {
        self.strobe = value & 1 != 0;

        if self.strobe {
            self.index = 0;
        }
    }

    fn read(&mut self) -> u8 {
        if self.strobe {
            return self.state & 1;
        }

        if self.index < 8 {
            let v = (self.state >> self.index) & 1;
            self.index += 1;
            v
        } else {
            1
        }
    }
}

struct Ppu {
    cart: Cartridge,

    vram: [u8; 2048],
    palette: [u8; 32],
    oam: [u8; 256],

    ctrl: u8,
    mask: u8,
    status: u8,
    oam_addr: u8,

    v: u16,
    t: u16,
    x: u8,
    w: u8,

    read_buffer: u8,

    scanline: i16,
    dot: i32,

    frame_ready: bool,
    nmi_pending: bool,
    nmi_fired: bool,

    sprite0_col: Option<usize>,
    sprite0_flagged: bool,

    bg_opaque: [u8; 256],

    framebuffer: Vec<u32>,
}

impl Ppu {
    fn new(cart: Cartridge) -> Self {
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
            nmi_fired: false,

            sprite0_col: None,
            sprite0_flagged: true,

            bg_opaque: [0; 256],

            framebuffer: vec![0; WIDTH * HEIGHT],
        }
    }

    fn nt_index(&self, addr: u16) -> usize {
        let rel = addr & 0x0fff;
        let table = (rel >> 10) as usize;
        let offset = (rel & 0x03ff) as usize;

        let physical = if self.cart.mirroring_vertical {
            table & 1
        } else {
            (table >> 1) & 1
        };

        physical * 0x400 + offset
    }

    fn palette_index(addr: u16) -> usize {
        let mut i = (addr & 0x1f) as usize;

        if i >= 0x10 && i % 4 == 0 {
            i -= 0x10;
        }

        i
    }

    fn internal_read(&self, addr: u16) -> u8 {
        let addr = addr & 0x3fff;

        if addr < 0x2000 {
            self.cart.chr_read(addr)
        } else if addr < 0x3f00 {
            self.vram[self.nt_index(addr)]
        } else {
            self.palette[Self::palette_index(addr)]
        }
    }

    fn internal_write(&mut self, addr: u16, value: u8) {
        let addr = addr & 0x3fff;

        if addr < 0x2000 {
            self.cart.chr_write(addr, value);
        } else if addr < 0x3f00 {
            let i = self.nt_index(addr);
            self.vram[i] = value;
        } else {
            self.palette[Self::palette_index(addr)] = value & 0x3f;
        }
    }

    fn cpu_read(&mut self, reg: u16) -> u8 {
        match reg {
            2 => {
                let r = self.status & 0xe0;

                self.status &= 0x7f;
                self.w = 0;

                r
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

                    self.read_buffer =
                        self.internal_read(addr.wrapping_sub(0x1000));
                }

                self.v = self.v.wrapping_add(
                    if self.ctrl & 4 != 0 {
                        32
                    } else {
                        1
                    }
                ) & 0x7fff;

                result
            }

            _ => 0,
        }
    }

    fn cpu_write(&mut self, reg: u16, value: u8) {
        match reg {
            0 => {
                self.ctrl = value;

                self.t =
                    (self.t & 0xf3ff) |
                    (((value & 3) as u16) << 10);
            }

            1 => self.mask = value,

            3 => self.oam_addr = value,

            4 => {
                self.oam[self.oam_addr as usize] = value;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }

            5 => {
                if self.w == 0 {
                    self.x = value & 7;

                    self.t =
                        (self.t & 0xffe0) |
                        ((value >> 3) as u16);

                    self.w = 1;
                } else {
                    self.t =
                        (self.t & 0x8fff) |
                        (((value & 7) as u16) << 12);

                    self.t =
                        (self.t & 0xfc1f) |
                        (((value & 0xf8) as u16) << 2);

                    self.w = 0;
                }
            }

            6 => {
                if self.w == 0 {
                    self.t =
                        (self.t & 0x80ff) |
                        (((value & 0x3f) as u16) << 8);

                    self.w = 1;
                } else {
                    self.t =
                        (self.t & 0xff00) |
                        value as u16;

                    self.v = self.t;
                    self.w = 0;
                }
            }

            7 => {
                let v = self.v & 0x3fff;

                self.internal_write(v, value);

                self.v = self.v.wrapping_add(
                    if self.ctrl & 4 != 0 {
                        32
                    } else {
                        1
                    }
                ) & 0x7fff;
            }

            _ => {}
        }
    }

    fn inc_vertical(mut v: u16) -> u16 {
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

    fn render_scanline(&mut self, y: usize) {
        let bg_show = self.mask & 0x08 != 0;
        let spr_show = self.mask & 0x10 != 0;

        let backdrop =
            NES_PALETTE[(self.palette[0] & 0x3f) as usize];

        if !bg_show {
            for x in 0..256 {
                self.bg_opaque[x] = 0;

                self.framebuffer[y * WIDTH + x] =
                    rgb(backdrop.0, backdrop.1, backdrop.2);
            }
        } else {
            let mut vv =
                (self.v & 0xfbe0) |
                (self.t & 0x041f);

            let fine_y =
                ((self.v >> 12) & 7) as u16;

            let pattern =
                if self.ctrl & 0x10 != 0 {
                    0x1000
                } else {
                    0
                };

            let mut pixel_x = -(self.x as i32);

            for _ in 0..33 {
                let coarse_x = vv & 0x1f;
                let coarse_y = (vv >> 5) & 0x1f;
                let nt_sel = (vv >> 10) & 3;

                let nt_base =
                    0x2000 + nt_sel * 0x400;

                let tile = self.internal_read(
                    nt_base +
                    coarse_y * 32 +
                    coarse_x
                );

                let attr_addr =
                    nt_base +
                    0x3c0 +
                    (coarse_y / 4) * 8 +
                    (coarse_x / 4);

                let attr =
                    self.internal_read(attr_addr);

                let shift =
                    ((coarse_y % 4) / 2) * 4 +
                    ((coarse_x % 4) / 2) * 2;

                let pal =
                    (attr >> shift) & 3;

                let p0 = self.internal_read(
                    pattern +
                    tile as u16 * 16 +
                    fine_y
                );

                let p1 = self.internal_read(
                    pattern +
                    tile as u16 * 16 +
                    fine_y +
                    8
                );

                for bit in 0..8 {
                    let px = pixel_x + bit;

                    if px < 0 || px >= 256 {
                        continue;
                    }

                    let b0 =
                        (p0 >> (7 - bit)) & 1;

                    let b1 =
                        (p1 >> (7 - bit)) & 1;

                    let ci =
                        (b1 << 1) | b0;

                    let px = px as usize;

                    if ci == 0 {
                        self.bg_opaque[px] = 0;

                        self.framebuffer[
                            y * WIDTH + px
                        ] =
                            rgb(
                                backdrop.0,
                                backdrop.1,
                                backdrop.2
                            );
                    } else {
                        self.bg_opaque[px] = 1;

                        let c = NES_PALETTE[
                            (self.internal_read(
                                0x3f00 +
                                pal as u16 * 4 +
                                ci as u16
                            ) & 0x3f) as usize
                        ];

                        self.framebuffer[
                            y * WIDTH + px
                        ] =
                            rgb(c.0, c.1, c.2);
                    }
                }

                pixel_x += 8;

                if coarse_x == 31 {
                    vv =
                        (vv & !0x001f) ^
                        0x0400;
                } else {
                    vv += 1;
                }
            }
        }

        if spr_show {
            self.render_sprites(y);
        }

        if bg_show {
            self.v = Self::inc_vertical(self.v);
        }
    }

    fn render_sprites(&mut self, y: usize) {
        let height =
            if self.ctrl & 0x20 != 0 {
                16
            } else {
                8
            };

        let mut found = Vec::new();

        for i in 0..64 {
            let sy =
                self.oam[i * 4] as i32;

            let row =
                y as i32 - sy;

            if row >= 0 && row < height {
                found.push(i);

                if found.len() == 8 {
                    self.status |= 0x20;
                    break;
                }
            }
        }

        for &i in found.iter().rev() {
            let sy =
                self.oam[i * 4] as i32;

            let mut row =
                y as i32 - sy;

            let tile =
                self.oam[i * 4 + 1];

            let attr =
                self.oam[i * 4 + 2];

            let sx =
                self.oam[i * 4 + 3] as i32;

            let vflip =
                attr & 0x80 != 0;

            let hflip =
                attr & 0x40 != 0;

            let behind =
                attr & 0x20 != 0;

            let pal =
                attr & 3;

            if vflip {
                row =
                    height - 1 - row;
            }

            let (pattern, tile_num, row) =
                if height == 16 {
                    let pattern =
                        if tile & 1 != 0 {
                            0x1000
                        } else {
                            0
                        };

                    let mut t =
                        tile & 0xfe;

                    let mut r =
                        row;

                    if r >= 8 {
                        t += 1;
                        r -= 8;
                    }

                    (pattern, t, r)
                } else {
                    (
                        if self.ctrl & 8 != 0 {
                            0x1000
                        } else {
                            0
                        },
                        tile,
                        row
                    )
                };

            let p0 =
                self.internal_read(
                    pattern +
                    tile_num as u16 * 16 +
                    row as u16
                );

            let p1 =
                self.internal_read(
                    pattern +
                    tile_num as u16 * 16 +
                    row as u16 +
                    8
                );

            for col in 0..8 {
                let bit =
                    if hflip {
                        col
                    } else {
                        7 - col
                    };

                let b0 =
                    (p0 >> bit) & 1;

                let b1 =
                    (p1 >> bit) & 1;

                let ci =
                    (b1 << 1) | b0;

                if ci == 0 {
                    continue;
                }

                let screen_x =
                    sx + col as i32;

                if !(0..256).contains(&screen_x) {
                    continue;
                }

                let screen_x =
                    screen_x as usize;

                if i == 0 &&
                   self.bg_opaque[screen_x] != 0 &&
                   screen_x != 255
                {
                    if self.sprite0_col
                        .map_or(true, |v| screen_x < v)
                    {
                        self.sprite0_col =
                            Some(screen_x);

                        self.sprite0_flagged =
                            false;
                    }
                }

                if behind &&
                   self.bg_opaque[screen_x] != 0
                {
                    continue;
                }

                let c =
                    NES_PALETTE[
                        (self.internal_read(
                            0x3f10 +
                            pal as u16 * 4 +
                            ci as u16
                        ) & 0x3f) as usize
                    ];

                self.framebuffer[
                    y * WIDTH + screen_x
                ] =
                    rgb(c.0, c.1, c.2);
            }
        }
    }

    fn catch_up(&mut self, mut dots: i32) {
        while dots > 0 {
            let space =
                341 - self.dot;

            let step =
                space.min(dots);

            let old_dot =
                self.dot;

            self.dot += step;
            dots -= step;

            if self.scanline >= 0 &&
               self.scanline < 240 &&
               !self.sprite0_flagged
            {
                if let Some(col) =
                    self.sprite0_col
                {
                    let threshold =
                        col as i32 + 2;

                    if old_dot <= threshold &&
                       threshold < self.dot
                    {
                        self.status |= 0x40;
                        self.sprite0_flagged =
                            true;
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
        }

        match self.scanline {
            0..=239 => {
                self.sprite0_col = None;
                self.sprite0_flagged = true;

                self.render_scanline(
                    self.scanline as usize
                );
            }

            241 => {
                self.status |= 0x80;
                self.nmi_fired = false;

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

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) |
    ((g as u32) << 8) |
    b as u32
}

struct Bus {
    ram: [u8; 2048],
    ppu: Ppu,
    controller: Controller,
}

impl Bus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1fff => {
                self.ram[
                    (addr & 0x07ff) as usize
                ]
            }

            0x2000..=0x3fff => {
                self.ppu.cpu_read(addr & 7)
            }

            0x4016 => {
                self.controller.read()
            }

            0x4017 => 0,

            0x4000..=0x401f => 0,

            0x4020..=0xffff => {
                self.ppu.cart.cpu_read(addr)
            }
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => {
                self.ram[
                    (addr & 0x07ff) as usize
                ] = value;
            }

            0x2000..=0x3fff => {
                self.ppu.cpu_write(
                    addr & 7,
                    value
                );
            }

            0x4014 => {
                let base =
                    (value as u16) << 8;

                let mut temp =
                    [0u8; 256];

                for i in 0..256 {
                    temp[i] =
                        self.read(
                            base + i as u16
                        );
                }

                for i in 0..256 {
                    self.ppu.oam[
                        (self.ppu.oam_addr as usize + i) & 255
                    ] = temp[i];
                }

                self.ppu.catch_up(
                    513 * 3
                );
            }

            0x4016 => {
                self.controller.write(value);
            }

            0x4000..=0x401f => {}

            0x4020..=0xffff => {
                self.ppu.cart.cpu_write(
                    addr,
                    value
                );
            }
        }
    }
}

const C: u8 = 0x01;
const Z: u8 = 0x02;
const I: u8 = 0x04;
const D: u8 = 0x08;
const B: u8 = 0x10;
const U: u8 = 0x20;
const V: u8 = 0x40;
const N: u8 = 0x80;

#[derive(Clone, Copy)]
enum Mode {
    Imp,
    Acc,
    Imm,
    Zp,
    Zpx,
    Zpy,
    Abs,
    Absx,
    Absy,
    Ind,
    Indx,
    Indy,
    Rel,
}

struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    pc: u16,
    status: u8,
    warned: [bool; 256],
}

impl Cpu {
    fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xfd,
            pc: 0,
            status: 0x24,
            warned: [false; 256],
        }
    }

    fn flag(&self, f: u8) -> bool {
        self.status & f != 0
    }

    fn set_flag(&mut self, f: u8, v: bool) {
        if v {
            self.status |= f;
        } else {
            self.status &= !f;
        }
    }

    fn zn(&mut self, v: u8) {
        self.set_flag(Z, v == 0);
        self.set_flag(N, v & 0x80 != 0);
    }

    fn reset(&mut self, bus: &mut Bus) {
        let lo =
            bus.read(0xfffc);

        let hi =
            bus.read(0xfffd);

        self.pc =
            u16::from_le_bytes([lo, hi]);

        self.sp = 0xfd;
        self.status = 0x24;

        self.a = 0;
        self.x = 0;
        self.y = 0;
    }

    fn push(&mut self, bus: &mut Bus, v: u8) {
        bus.write(
            0x0100 + self.sp as u16,
            v
        );

        self.sp =
            self.sp.wrapping_sub(1);
    }

    fn pop(&mut self, bus: &mut Bus) -> u8 {
        self.sp =
            self.sp.wrapping_add(1);

        bus.read(
            0x0100 + self.sp as u16
        )
    }

    fn nmi(&mut self, bus: &mut Bus) {
        self.push(
            bus,
            (self.pc >> 8) as u8
        );

        self.push(
            bus,
            self.pc as u8
        );

        self.push(
            bus,
            (self.status & !B) | U
        );

        self.set_flag(I, true);

        let lo =
            bus.read(0xfffa);

        let hi =
            bus.read(0xfffb);

        self.pc =
            u16::from_le_bytes([lo, hi]);
    }

    fn fetch(&mut self, bus: &mut Bus) -> u8 {
        let v =
            bus.read(self.pc);

        self.pc =
            self.pc.wrapping_add(1);

        v
    }

    fn word(&mut self, bus: &mut Bus) -> u16 {
        let lo =
            self.fetch(bus);

        let hi =
            self.fetch(bus);

        u16::from_le_bytes([lo, hi])
    }

    fn operand(
        &mut self,
        bus: &mut Bus,
        mode: Mode
    ) -> Option<u16> {
        match mode {
            Mode::Imm => {
                let a = self.pc;

                self.pc =
                    self.pc.wrapping_add(1);

                Some(a)
            }

            Mode::Zp =>
                Some(
                    self.fetch(bus) as u16
                ),

            Mode::Zpx =>
                Some(
                    self.fetch(bus)
                        .wrapping_add(self.x)
                        as u16
                ),

            Mode::Zpy =>
                Some(
                    self.fetch(bus)
                        .wrapping_add(self.y)
                        as u16
                ),

            Mode::Abs =>
                Some(self.word(bus)),

            Mode::Absx =>
                Some(
                    self.word(bus)
                        .wrapping_add(self.x as u16)
                ),

            Mode::Absy =>
                Some(
                    self.word(bus)
                        .wrapping_add(self.y as u16)
                ),

            Mode::Ind => {
                let ptr =
                    self.word(bus);

                let lo =
                    bus.read(ptr);

                let hi_addr =
                    if ptr & 0xff == 0xff {
                        ptr & 0xff00
                    } else {
                        ptr + 1
                    };

                let hi =
                    bus.read(hi_addr);

                Some(
                    u16::from_le_bytes([lo, hi])
                )
            }

            Mode::Indx => {
                let zp =
                    self.fetch(bus)
                        .wrapping_add(self.x);

                let lo =
                    bus.read(zp as u16);

                let hi =
                    bus.read(
                        zp.wrapping_add(1) as u16
                    );

                Some(
                    u16::from_le_bytes([lo, hi])
                )
            }

            Mode::Indy => {
                let zp =
                    self.fetch(bus);

                let lo =
                    bus.read(zp as u16);

                let hi =
                    bus.read(
                        zp.wrapping_add(1) as u16
                    );

                Some(
                    u16::from_le_bytes([lo, hi])
                        .wrapping_add(self.y as u16)
                )
            }

            Mode::Rel => {
                let off =
                    self.fetch(bus) as i8;

                Some(
                    self.pc.wrapping_add_signed(
                        off as i16
                    )
                )
            }

            Mode::Imp |
            Mode::Acc => None,
        }
    }

    fn load(
        &mut self,
        bus: &mut Bus,
        mode: Mode,
        addr: Option<u16>
    ) -> u8 {
        if matches!(mode, Mode::Acc) {
            self.a
        } else {
            bus.read(addr.unwrap())
        }
    }

    fn store(
        &mut self,
        bus: &mut Bus,
        mode: Mode,
        addr: Option<u16>,
        value: u8
    ) {
        if matches!(mode, Mode::Acc) {
            self.a = value;
        } else {
            bus.write(
                addr.unwrap(),
                value
            );
        }
    }

    fn adc(&mut self, v: u8) {
        let a = self.a;

        let carry =
            if self.flag(C) {
                1u16
            } else {
                0
            };

        let result =
            a as u16 +
            v as u16 +
            carry;

        let r =
            result as u8;

        self.set_flag(
            C,
            result > 0xff
        );

        self.set_flag(
            V,
            (!(a ^ v) &
             (a ^ r) &
             0x80) != 0
        );

        self.a = r;
        self.zn(r);
    }

    fn sbc(&mut self, v: u8) {
        self.adc(v ^ 0xff);
    }

    fn step(&mut self, bus: &mut Bus) -> u32 {
        let opcode =
            self.fetch(bus);

        let (name, mode, cycles) =
            opcode_info(opcode);

        if name == "ILL" {
            if !self.warned[opcode as usize] {
                println!(
                    "Warning: unsupported opcode {:02X} at PC={:04X}; treating as NOP",
                    opcode,
                    self.pc.wrapping_sub(1)
                );

                self.warned[
                    opcode as usize
                ] = true;
            }

            return 2;
        }

        let addr =
            if !matches!(
                mode,
                Mode::Imp | Mode::Acc
            ) {
                self.operand(bus, mode)
            } else {
                None
            };

        match name {
            // FIXED E0499:
            // Load the operand into a local first so that the
            // mutable borrow used by load() ends before adc().
            "ADC" => {
                let v =
                    self.load(bus, mode, addr);

                self.adc(v);
            }

            "AND" => {
                self.a &=
                    self.load(bus, mode, addr);

                self.zn(self.a);
            }

            "ASL" => {
                let v =
                    self.load(bus, mode, addr);

                self.set_flag(
                    C,
                    v & 0x80 != 0
                );

                let r =
                    v << 1;

                self.store(
                    bus,
                    mode,
                    addr,
                    r
                );

                self.zn(r);
            }

            "BCC" =>
                if !self.flag(C) {
                    self.pc =
                        addr.unwrap()
                },

            "BCS" =>
                if self.flag(C) {
                    self.pc =
                        addr.unwrap()
                },

            "BEQ" =>
                if self.flag(Z) {
                    self.pc =
                        addr.unwrap()
                },

            "BNE" =>
                if !self.flag(Z) {
                    self.pc =
                        addr.unwrap()
                },

            "BMI" =>
                if self.flag(N) {
                    self.pc =
                        addr.unwrap()
                },

            "BPL" =>
                if !self.flag(N) {
                    self.pc =
                        addr.unwrap()
                },

            "BVC" =>
                if !self.flag(V) {
                    self.pc =
                        addr.unwrap()
                },

            "BVS" =>
                if self.flag(V) {
                    self.pc =
                        addr.unwrap()
                },

            "BIT" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.set_flag(
                    Z,
                    self.a & v == 0
                );

                self.set_flag(
                    V,
                    v & 0x40 != 0
                );

                self.set_flag(
                    N,
                    v & 0x80 != 0
                );
            }

            "BRK" => {
                self.pc =
                    self.pc.wrapping_add(1);

                self.push(
                    bus,
                    (self.pc >> 8) as u8
                );

                self.push(
                    bus,
                    self.pc as u8
                );

                self.push(
                    bus,
                    self.status | B | U
                );

                self.set_flag(I, true);

                let lo =
                    bus.read(0xfffe);

                let hi =
                    bus.read(0xffff);

                self.pc =
                    u16::from_le_bytes([lo, hi]);
            }

            "CLC" =>
                self.set_flag(C, false),

            "CLD" =>
                self.set_flag(D, false),

            "CLI" =>
                self.set_flag(I, false),

            "CLV" =>
                self.set_flag(V, false),

            "CMP" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.set_flag(
                    C,
                    self.a >= v
                );

                self.zn(
                    self.a.wrapping_sub(v)
                );
            }

            "CPX" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.set_flag(
                    C,
                    self.x >= v
                );

                self.zn(
                    self.x.wrapping_sub(v)
                );
            }

            "CPY" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.set_flag(
                    C,
                    self.y >= v
                );

                self.zn(
                    self.y.wrapping_sub(v)
                );
            }

            "DEC" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    )
                    .wrapping_sub(1);

                self.store(
                    bus,
                    mode,
                    addr,
                    v
                );

                self.zn(v);
            }

            "DEX" => {
                self.x =
                    self.x.wrapping_sub(1);

                self.zn(self.x);
            }

            "DEY" => {
                self.y =
                    self.y.wrapping_sub(1);

                self.zn(self.y);
            }

            "EOR" => {
                self.a ^=
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.zn(self.a);
            }

            "INC" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    )
                    .wrapping_add(1);

                self.store(
                    bus,
                    mode,
                    addr,
                    v
                );

                self.zn(v);
            }

            "INX" => {
                self.x =
                    self.x.wrapping_add(1);

                self.zn(self.x);
            }

            "INY" => {
                self.y =
                    self.y.wrapping_add(1);

                self.zn(self.y);
            }

            "JMP" =>
                self.pc =
                    addr.unwrap(),

            "JSR" => {
                let ret =
                    self.pc.wrapping_sub(1);

                self.push(
                    bus,
                    (ret >> 8) as u8
                );

                self.push(
                    bus,
                    ret as u8
                );

                self.pc =
                    addr.unwrap();
            }

            "LDA" => {
                self.a =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.zn(self.a);
            }

            "LDX" => {
                self.x =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.zn(self.x);
            }

            "LDY" => {
                self.y =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.zn(self.y);
            }

            "LSR" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.set_flag(
                    C,
                    v & 1 != 0
                );

                let r =
                    v >> 1;

                self.store(
                    bus,
                    mode,
                    addr,
                    r
                );

                self.zn(r);
            }

            "NOP" => {}

            "ORA" => {
                self.a |=
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                self.zn(self.a);
            }

            "PHA" =>
                self.push(bus, self.a),

            "PHP" =>
                self.push(
                    bus,
                    self.status | B | U
                ),

            "PLA" => {
                self.a =
                    self.pop(bus);

                self.zn(self.a);
            }

            "PLP" => {
                self.status =
                    (self.pop(bus) & !B) | U;
            }

            "ROL" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                let ci =
                    if self.flag(C) {
                        1
                    } else {
                        0
                    };

                self.set_flag(
                    C,
                    v & 0x80 != 0
                );

                let r =
                    (v << 1) | ci;

                self.store(
                    bus,
                    mode,
                    addr,
                    r
                );

                self.zn(r);
            }

            "ROR" => {
                let v =
                    self.load(
                        bus,
                        mode,
                        addr
                    );

                let ci =
                    if self.flag(C) {
                        0x80
                    } else {
                        0
                    };

                self.set_flag(
                    C,
                    v & 1 != 0
                );

                let r =
                    (v >> 1) | ci;

                self.store(
                    bus,
                    mode,
                    addr,
                    r
                );

                self.zn(r);
            }

            "RTI" => {
                self.status =
                    (self.pop(bus) & !B) | U;

                let lo =
                    self.pop(bus);

                let hi =
                    self.pop(bus);

                self.pc =
                    u16::from_le_bytes([lo, hi]);
            }

            "RTS" => {
                let lo =
                    self.pop(bus);

                let hi =
                    self.pop(bus);

                self.pc =
                    u16::from_le_bytes([lo, hi])
                        .wrapping_add(1);
            }

            // FIXED E0499:
            // Same fix as ADC: evaluate load() first.
            "SBC" => {
                let v =
                    self.load(bus, mode, addr);

                self.sbc(v);
            }

            "SEC" =>
                self.set_flag(C, true),

            "SED" =>
                self.set_flag(D, true),

            "SEI" =>
                self.set_flag(I, true),

            "STA" =>
                bus.write(
                    addr.unwrap(),
                    self.a
                ),

            "STX" =>
                bus.write(
                    addr.unwrap(),
                    self.x
                ),

            "STY" =>
                bus.write(
                    addr.unwrap(),
                    self.y
                ),

            "TAX" => {
                self.x =
                    self.a;

                self.zn(self.x);
            }

            "TAY" => {
                self.y =
                    self.a;

                self.zn(self.y);
            }

            "TSX" => {
                self.x =
                    self.sp;

                self.zn(self.x);
            }

            "TXA" => {
                self.a =
                    self.x;

                self.zn(self.a);
            }

            "TXS" =>
                self.sp =
                    self.x,

            "TYA" => {
                self.a =
                    self.y;

                self.zn(self.a);
            }

            _ => unreachable!(),
        }

        cycles
    }
}

fn opcode_info(op: u8) -> (&'static str, Mode, u32) {
    use Mode::*;

    match op {
        0x69 => ("ADC",Imm,2),
        0x65 => ("ADC",Zp,3),
        0x75 => ("ADC",Zpx,4),
        0x6d => ("ADC",Abs,4),
        0x7d => ("ADC",Absx,4),
        0x79 => ("ADC",Absy,4),
        0x61 => ("ADC",Indx,6),
        0x71 => ("ADC",Indy,5),

        0x29 => ("AND",Imm,2),
        0x25 => ("AND",Zp,3),
        0x35 => ("AND",Zpx,4),
        0x2d => ("AND",Abs,4),
        0x3d => ("AND",Absx,4),
        0x39 => ("AND",Absy,4),
        0x21 => ("AND",Indx,6),
        0x31 => ("AND",Indy,5),

        0x0a => ("ASL",Acc,2),
        0x06 => ("ASL",Zp,5),
        0x16 => ("ASL",Zpx,6),
        0x0e => ("ASL",Abs,6),
        0x1e => ("ASL",Absx,7),

        0x90 => ("BCC",Rel,2),
        0xb0 => ("BCS",Rel,2),
        0xf0 => ("BEQ",Rel,2),
        0x30 => ("BMI",Rel,2),
        0xd0 => ("BNE",Rel,2),
        0x10 => ("BPL",Rel,2),
        0x50 => ("BVC",Rel,2),
        0x70 => ("BVS",Rel,2),

        0x24 => ("BIT",Zp,3),
        0x2c => ("BIT",Abs,4),

        0x00 => ("BRK",Imp,7),

        0x18 => ("CLC",Imp,2),
        0xd8 => ("CLD",Imp,2),
        0x58 => ("CLI",Imp,2),
        0xb8 => ("CLV",Imp,2),

        0xc9 => ("CMP",Imm,2),
        0xc5 => ("CMP",Zp,3),
        0xd5 => ("CMP",Zpx,4),
        0xcd => ("CMP",Abs,4),
        0xdd => ("CMP",Absx,4),
        0xd9 => ("CMP",Absy,4),
        0xc1 => ("CMP",Indx,6),
        0xd1 => ("CMP",Indy,5),

        0xe0 => ("CPX",Imm,2),
        0xe4 => ("CPX",Zp,3),
        0xec => ("CPX",Abs,4),

        0xc0 => ("CPY",Imm,2),
        0xc4 => ("CPY",Zp,3),
        0xcc => ("CPY",Abs,4),

        0xc6 => ("DEC",Zp,5),
        0xd6 => ("DEC",Zpx,6),
        0xce => ("DEC",Abs,6),
        0xde => ("DEC",Absx,7),

        0xca => ("DEX",Imp,2),
        0x88 => ("DEY",Imp,2),

        0x49 => ("EOR",Imm,2),
        0x45 => ("EOR",Zp,3),
        0x55 => ("EOR",Zpx,4),
        0x4d => ("EOR",Abs,4),
        0x5d => ("EOR",Absx,4),
        0x59 => ("EOR",Absy,4),
        0x41 => ("EOR",Indx,6),
        0x51 => ("EOR",Indy,5),

        0xe6 => ("INC",Zp,5),
        0xf6 => ("INC",Zpx,6),
        0xee => ("INC",Abs,6),
        0xfe => ("INC",Absx,7),

        0xe8 => ("INX",Imp,2),
        0xc8 => ("INY",Imp,2),

        0x4c => ("JMP",Abs,3),
        0x6c => ("JMP",Ind,5),

        0x20 => ("JSR",Abs,6),

        0xa9 => ("LDA",Imm,2),
        0xa5 => ("LDA",Zp,3),
        0xb5 => ("LDA",Zpx,4),
        0xad => ("LDA",Abs,4),
        0xbd => ("LDA",Absx,4),
        0xb9 => ("LDA",Absy,4),
        0xa1 => ("LDA",Indx,6),
        0xb1 => ("LDA",Indy,5),

        0xa2 => ("LDX",Imm,2),
        0xa6 => ("LDX",Zp,3),
        0xb6 => ("LDX",Zpy,4),
        0xae => ("LDX",Abs,4),
        0xbe => ("LDX",Absy,4),

        0xa0 => ("LDY",Imm,2),
        0xa4 => ("LDY",Zp,3),
        0xb4 => ("LDY",Zpx,4),
        0xac => ("LDY",Abs,4),
        0xbc => ("LDY",Absx,4),

        0x4a => ("LSR",Acc,2),
        0x46 => ("LSR",Zp,5),
        0x56 => ("LSR",Zpx,6),
        0x4e => ("LSR",Abs,6),
        0x5e => ("LSR",Absx,7),

        0xea => ("NOP",Imp,2),

        0x09 => ("ORA",Imm,2),
        0x05 => ("ORA",Zp,3),
        0x15 => ("ORA",Zpx,4),
        0x0d => ("ORA",Abs,4),
        0x1d => ("ORA",Absx,4),
        0x19 => ("ORA",Absy,4),
        0x01 => ("ORA",Indx,6),
        0x11 => ("ORA",Indy,5),

        0x48 => ("PHA",Imp,3),
        0x08 => ("PHP",Imp,3),
        0x68 => ("PLA",Imp,4),
        0x28 => ("PLP",Imp,4),

        0x2a => ("ROL",Acc,2),
        0x26 => ("ROL",Zp,5),
        0x36 => ("ROL",Zpx,6),
        0x2e => ("ROL",Abs,6),
        0x3e => ("ROL",Absx,7),

        0x6a => ("ROR",Acc,2),
        0x66 => ("ROR",Zp,5),
        0x76 => ("ROR",Zpx,6),
        0x6e => ("ROR",Abs,6),
        0x7e => ("ROR",Absx,7),

        0x40 => ("RTI",Imp,6),
        0x60 => ("RTS",Imp,6),

        0xe9 => ("SBC",Imm,2),
        0xe5 => ("SBC",Zp,3),
        0xf5 => ("SBC",Zpx,4),
        0xed => ("SBC",Abs,4),
        0xfd => ("SBC",Absx,4),
        0xf9 => ("SBC",Absy,4),
        0xe1 => ("SBC",Indx,6),
        0xf1 => ("SBC",Indy,5),

        0x38 => ("SEC",Imp,2),
        0xf8 => ("SED",Imp,2),
        0x78 => ("SEI",Imp,2),

        0x85 => ("STA",Zp,3),
        0x95 => ("STA",Zpx,4),
        0x8d => ("STA",Abs,4),
        0x9d => ("STA",Absx,5),
        0x99 => ("STA",Absy,5),
        0x81 => ("STA",Indx,6),
        0x91 => ("STA",Indy,6),

        0x86 => ("STX",Zp,3),
        0x96 => ("STX",Zpy,4),
        0x8e => ("STX",Abs,4),

        0x84 => ("STY",Zp,3),
        0x94 => ("STY",Zpx,4),
        0x8c => ("STY",Abs,4),

        0xaa => ("TAX",Imp,2),
        0xa8 => ("TAY",Imp,2),
        0xba => ("TSX",Imp,2),
        0x8a => ("TXA",Imp,2),
        0x9a => ("TXS",Imp,2),
        0x98 => ("TYA",Imp,2),

        _ => ("ILL",Imp,2),
    }
}

fn main() {
    let args: Vec<String> =
        env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: nes <rom.nes>"
        );

        return;
    }

    let rom =
        &args[1];

    let cart =
        match Cartridge::load(rom) {
            Ok(c) => c,

            Err(e) => {
                eprintln!(
                    "Error: {}",
                    e
                );

                return;
            }
        };

    let controller =
        Controller::new();

    let mut bus =
        Bus {
            ram: [0; 2048],
            ppu: Ppu::new(cart),
            controller,
        };

    let mut cpu =
        Cpu::new();

    cpu.reset(&mut bus);

    let mut window =
        Window::new(
            "NES",
            WIDTH * 3,
            HEIGHT * 3,
            WindowOptions {
                resize: false,
                scale: minifb::Scale::X1,
                ..WindowOptions::default()
            },
        )
        .expect(
            "Could not create window"
        );

    window.limit_update_rate(
        Some(Duration::from_micros(16_667))
    );

    let max_frames =
        env::var("NES_MAX_FRAMES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok());

    let mut frame_count =
        0u64;

    let mut last =
        Instant::now();

    while window.is_open() &&
          !window.is_key_down(Key::Escape)
    {
        bus.controller.update(
            &window
        );

        while !bus.ppu.frame_ready {
            let cycles =
                cpu.step(&mut bus);

            bus.ppu.catch_up(
                (cycles * 3) as i32
            );

            if bus.ppu.nmi_pending &&
               !bus.ppu.nmi_fired
            {
                bus.ppu.nmi_pending =
                    false;

                bus.ppu.nmi_fired =
                    true;

                cpu.nmi(&mut bus);
            }
        }

        bus.ppu.frame_ready =
            false;

        frame_count += 1;

        window
            .update_with_buffer(
                &bus.ppu.framebuffer,
                WIDTH,
                HEIGHT
            )
            .expect(
                "Failed to update window"
            );

        if let Some(max) =
            max_frames
        {
            if frame_count >= max {
                break;
            }
        }

        let elapsed =
            last.elapsed();

        if elapsed <
           Duration::from_micros(16_667)
        {
            std::thread::sleep(
                Duration::from_micros(16_667)
                    - elapsed
            );
        }

        last =
            Instant::now();
    }
}
