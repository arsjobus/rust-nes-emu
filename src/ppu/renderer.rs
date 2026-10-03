use super::{Ppu, WIDTH};

pub const NES_PALETTE: [(u8, u8, u8); 64] = [
    (84, 84, 84),
    (0, 30, 116),
    (8, 16, 144),
    (48, 0, 136),
    (68, 0, 100),
    (92, 0, 48),
    (84, 4, 0),
    (60, 24, 0),
    (32, 42, 0),
    (8, 58, 0),
    (0, 64, 0),
    (0, 60, 0),
    (0, 50, 60),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (152, 150, 152),
    (8, 76, 196),
    (48, 50, 236),
    (92, 30, 228),
    (136, 20, 176),
    (160, 20, 100),
    (152, 34, 32),
    (120, 60, 0),
    (84, 90, 0),
    (40, 114, 0),
    (8, 124, 0),
    (0, 118, 40),
    (0, 102, 120),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (76, 154, 236),
    (120, 124, 236),
    (176, 98, 236),
    (228, 84, 236),
    (236, 88, 180),
    (236, 106, 100),
    (212, 136, 32),
    (160, 170, 0),
    (116, 196, 0),
    (76, 208, 32),
    (56, 204, 108),
    (56, 180, 204),
    (60, 60, 60),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (168, 204, 236),
    (188, 188, 236),
    (212, 178, 236),
    (236, 174, 236),
    (236, 174, 212),
    (236, 180, 176),
    (228, 196, 144),
    (204, 210, 120),
    (180, 222, 120),
    (168, 226, 144),
    (152, 226, 180),
    (160, 214, 228),
    (160, 162, 160),
    (0, 0, 0),
    (0, 0, 0),
];

impl Ppu {
    /// Looks up a system palette entry, honouring the grayscale bit
    /// of $2001.
    fn system_color(&self, index: u8) -> (u8, u8, u8) {
        let mut index = index & 0x3f;

        if self.mask & 0x01 != 0 {
            index &= 0x30;
        }

        NES_PALETTE[index as usize]
    }

    pub(crate) fn render_scanline(&mut self, y: usize) {
        let background_enabled = self.mask & 0x08 != 0;

        let sprites_enabled = self.mask & 0x10 != 0;

        let backdrop = self.system_color(self.palette[0]);

        if !background_enabled {
            for x in 0..256 {
                self.bg_opaque[x] = 0;

                self.framebuffer[y * WIDTH + x] = rgb(backdrop.0, backdrop.1, backdrop.2);
            }
        } else {
            let mut v = (self.v & 0xfbe0) | (self.t & 0x041f);

            let fine_y = ((self.v >> 12) & 7) as u16;

            let pattern = if self.ctrl & 0x10 != 0 { 0x1000 } else { 0 };

            let mut pixel_x = -(self.x as i32);

            for tile_x in 0..33 {
                let coarse_x = v & 0x1f;

                let coarse_y = (v >> 5) & 0x1f;

                let nametable = (v >> 10) & 3;

                let nametable_base = 0x2000 + nametable * 0x400;

                let mut tile = self.internal_read(nametable_base + coarse_y * 32 + coarse_x);

                let attribute_addr = nametable_base + 0x3c0 + (coarse_y / 4) * 8 + (coarse_x / 4);

                let attribute = self.internal_read(attribute_addr);

                let shift = ((coarse_y % 4) / 2) * 4 + ((coarse_x % 4) / 2) * 2;

                let mut palette = (attribute >> shift) & 3;
                if let Some((split_tile, split_palette, _)) = self.cart.bg_tile(tile_x, y) {
                    tile = split_tile;
                    palette = split_palette;
                } else if let Some(extended_palette) =
                    self.cart
                        .bg_palette(tile_x, y, (coarse_x as usize, coarse_y as usize))
                {
                    palette = extended_palette;
                }

                let p0 = self.cart.bg_chr_read(
                    pattern + tile as u16 * 16 + fine_y,
                    tile_x,
                    y,
                    self.ctrl & 0x20 != 0,
                    (coarse_x as usize, coarse_y as usize),
                );

                let p1 = self.cart.bg_chr_read(
                    pattern + tile as u16 * 16 + fine_y + 8,
                    tile_x,
                    y,
                    self.ctrl & 0x20 != 0,
                    (coarse_x as usize, coarse_y as usize),
                );

                for bit in 0..8 {
                    let px = pixel_x + bit;

                    if px < 0 || px >= 256 {
                        continue;
                    }

                    let bit0 = (p0 >> (7 - bit)) & 1;

                    let bit1 = (p1 >> (7 - bit)) & 1;

                    let mut color_index = (bit1 << 1) | bit0;

                    // $2001 bit 1 clear hides the background in the
                    // leftmost 8 pixels.
                    if px < 8 && self.mask & 0x02 == 0 {
                        color_index = 0;
                    }

                    let px = px as usize;

                    if color_index == 0 {
                        self.bg_opaque[px] = 0;

                        self.framebuffer[y * WIDTH + px] = rgb(backdrop.0, backdrop.1, backdrop.2);
                    } else {
                        self.bg_opaque[px] = 1;

                        let color = self.system_color(
                            self.internal_read(0x3f00 + palette as u16 * 4 + color_index as u16),
                        );

                        self.framebuffer[y * WIDTH + px] = rgb(color.0, color.1, color.2);
                    }
                }

                pixel_x += 8;

                if coarse_x == 31 {
                    v = (v & !0x001f) ^ 0x0400;
                } else {
                    v += 1;
                }
            }
        }

        if sprites_enabled {
            self.render_sprites(y);
        }

        // The vertical scroll increment happens whenever rendering
        // is enabled, whether by the background or by sprites alone.
        if background_enabled || sprites_enabled {
            self.v = Self::inc_vertical(self.v);
        }
    }

    /// Dot at which sprite evaluation for scanline `line` raises the
    /// sprite overflow flag, if it does. This reproduces the hardware's
    /// buggy search: once eight sprites have been found, the PPU keeps
    /// scanning OAM but increments the byte offset within each entry
    /// together with the entry number, so it compares the wrong bytes
    /// (tile, attribute, X) against the scanline for most entries,
    /// giving both false positives and false negatives.
    pub(crate) fn overflow_set_dot(&self, line: i32) -> Option<i32> {
        let height = if self.ctrl & 0x20 != 0 { 16 } else { 8 };
        let in_range = |value: u8| (0..height).contains(&(line - (value as i32 + 1)));

        // Evaluation runs on dots 65-256, one OAM byte per two dots;
        // copying an in-range sprite takes three more reads.
        let mut dot = 65;
        let mut found = 0;
        let mut n = 0usize;

        while n < 64 && found < 8 {
            if in_range(self.oam[n * 4]) {
                found += 1;
                dot += 8;
            } else {
                dot += 2;
            }
            n += 1;
        }

        if found < 8 {
            return None;
        }

        let mut m = 0usize;
        while n < 64 {
            if in_range(self.oam[n * 4 + m]) {
                return Some(dot);
            }
            dot += 2;
            n += 1;
            m = (m + 1) & 3;
        }

        None
    }

    fn render_sprites(&mut self, y: usize) {
        let height = if self.ctrl & 0x20 != 0 { 16 } else { 8 };

        // At most 8 sprites are drawn per line; fixed storage avoids two
        // heap allocations per scanline.
        let mut visible = [0usize; 8];
        let mut visible_len = 0;

        for index in 0..64 {
            let sprite_y = self.oam[index * 4] as i32;

            // Sprite data is delayed by one scanline: a sprite
            // whose OAM Y is N first appears on scanline N + 1
            // (which is why software writes Y - 1 to OAM). So the
            // sprite's first row is drawn when `y == sprite_y + 1`.
            let row = y as i32 - (sprite_y + 1);

            if row >= 0 && row < height {
                // Only 8 sprites fit on a scanline; the overflow flag
                // is handled separately (see `overflow_set_dot`).
                if visible_len == 8 {
                    break;
                }

                visible[visible_len] = index;
                visible_len += 1;
            }
        }

        // Fetch every visible sprite's pattern bytes first, in
        // ascending OAM order - the same order the real PPU fetches
        // them in. This matters for mappers like MMC2 whose CHR
        // bank is chosen by a latch that flips based on the exact
        // sequence of pattern-table reads: some games (Punch-Out!!
        // in particular) rely on sprites being fetched in OAM order
        // so an earlier sprite's tile can flip the latch before a
        // later one is fetched. Drawing still happens in reverse
        // order afterwards, using these pre-fetched bytes, so
        // on-screen sprite priority (lower OAM index wins overlaps)
        // is unaffected.
        #[derive(Clone, Copy)]
        struct PreparedSprite {
            index: usize,
            sprite_x: i32,
            horizontal_flip: bool,
            behind_background: bool,
            palette: u8,
            p0: u8,
            p1: u8,
        }

        let mut prepared: [Option<PreparedSprite>; 8] = [None; 8];

        for (slot, &index) in visible[..visible_len].iter().enumerate() {
            let sprite_y = self.oam[index * 4] as i32;

            // Same one-scanline delay as in the range check above.
            let mut row = y as i32 - (sprite_y + 1);

            let tile = self.oam[index * 4 + 1];

            let attributes = self.oam[index * 4 + 2];

            let sprite_x = self.oam[index * 4 + 3] as i32;

            let vertical_flip = attributes & 0x80 != 0;

            let horizontal_flip = attributes & 0x40 != 0;

            let behind_background = attributes & 0x20 != 0;

            let palette = attributes & 3;

            if vertical_flip {
                row = height - 1 - row;
            }

            let (pattern, tile_number, row) = if height == 16 {
                let pattern = if tile & 1 != 0 { 0x1000 } else { 0 };

                let mut tile_number = tile & 0xfe;

                let mut row = row;

                if row >= 8 {
                    tile_number += 1;
                    row -= 8;
                }

                (pattern, tile_number, row)
            } else {
                (if self.ctrl & 8 != 0 { 0x1000 } else { 0 }, tile, row)
            };

            let p0 = self
                .cart
                .sprite_chr_read(pattern + tile_number as u16 * 16 + row as u16);

            let p1 = self
                .cart
                .sprite_chr_read(pattern + tile_number as u16 * 16 + row as u16 + 8);

            prepared[slot] = Some(PreparedSprite {
                index,
                sprite_x,
                horizontal_flip,
                behind_background,
                palette,
                p0,
                p1,
            });
        }

        // Sprite-vs-sprite priority: the lowest OAM index that has an
        // opaque pixel at a column wins that column, *even if it is a
        // "behind background" sprite that ends up hidden by the
        // background*. Later sprites must not show through it. So walk
        // in OAM order and claim columns as they are taken.
        let mut claimed = [false; 256];

        for sprite in prepared.iter().flatten() {
            let index = sprite.index;
            let sprite_x = sprite.sprite_x;
            let horizontal_flip = sprite.horizontal_flip;
            let behind_background = sprite.behind_background;
            let palette = sprite.palette;
            let p0 = sprite.p0;
            let p1 = sprite.p1;

            for column in 0..8 {
                let bit = if horizontal_flip { column } else { 7 - column };

                let bit0 = (p0 >> bit) & 1;

                let bit1 = (p1 >> bit) & 1;

                let color_index = (bit1 << 1) | bit0;

                if color_index == 0 {
                    continue;
                }

                let screen_x = sprite_x + column as i32;

                if !(0..256).contains(&screen_x) {
                    continue;
                }

                let screen_x = screen_x as usize;

                // $2001 bit 2 clear hides sprites in the leftmost 8
                // pixels (and suppresses sprite 0 hits there).
                if screen_x < 8 && self.mask & 0x04 == 0 {
                    continue;
                }

                if index == 0 && self.bg_opaque[screen_x] != 0 && screen_x != 255 {
                    if self.sprite0_col.map_or(true, |v| screen_x < v) {
                        self.sprite0_col = Some(screen_x);

                        self.sprite0_flagged = false;
                    }
                }

                if claimed[screen_x] {
                    continue;
                }

                claimed[screen_x] = true;

                if behind_background && self.bg_opaque[screen_x] != 0 {
                    continue;
                }

                let color = self.system_color(
                    self.internal_read(0x3f10 + palette as u16 * 4 + color_index as u16),
                );

                self.framebuffer[y * WIDTH + screen_x] = rgb(color.0, color.1, color.2);
            }
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

#[cfg(test)]
#[path = "../../tests/ppu/renderer.rs"]
mod tests;
