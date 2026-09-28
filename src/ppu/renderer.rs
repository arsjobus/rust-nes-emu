use super::{Ppu, HEIGHT, WIDTH};

pub const NES_PALETTE: [(u8, u8, u8); 64] = [
    (84,84,84),(0,30,116),(8,16,144),(48,0,136),
    (68,0,100),(92,0,48),(84,4,0),(60,24,0),
    (32,42,0),(8,58,0),(0,64,0),(0,60,0),
    (0,50,60),(0,0,0),(0,0,0),(0,0,0),

    (152,150,152),(8,76,196),(48,50,236),
    (92,30,228),(136,20,176),(160,20,100),
    (152,34,32),(120,60,0),(84,90,0),
    (40,114,0),(8,124,0),(0,118,40),
    (0,102,120),(0,0,0),(0,0,0),(0,0,0),

    (236,238,236),(76,154,236),(120,124,236),
    (176,98,236),(228,84,236),(236,88,180),
    (236,106,100),(212,136,32),(160,170,0),
    (116,196,0),(76,208,32),(56,204,108),
    (56,180,204),(60,60,60),(0,0,0),(0,0,0),

    (236,238,236),(168,204,236),(188,188,236),
    (212,178,236),(236,174,236),(236,174,212),
    (236,180,176),(228,196,144),(204,210,120),
    (180,222,120),(168,226,144),(152,226,180),
    (160,214,228),(160,162,160),(0,0,0),(0,0,0),
];

impl Ppu {
    pub(crate) fn render_scanline(
        &mut self,
        y: usize,
    ) {
        let background_enabled =
            self.mask & 0x08 != 0;

        let sprites_enabled =
            self.mask & 0x10 != 0;

        let backdrop =
            NES_PALETTE[
                (self.palette[0] & 0x3f) as usize
            ];

        if !background_enabled {
            for x in 0..256 {
                self.bg_opaque[x] = 0;

                self.framebuffer[
                    y * WIDTH + x
                ] = rgb(
                    backdrop.0,
                    backdrop.1,
                    backdrop.2,
                );
            }
        } else {
            let mut v =
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

            let mut pixel_x =
                -(self.x as i32);

            for _ in 0..33 {
                let coarse_x =
                    v & 0x1f;

                let coarse_y =
                    (v >> 5) & 0x1f;

                let nametable =
                    (v >> 10) & 3;

                let nametable_base =
                    0x2000 +
                    nametable * 0x400;

                let tile =
                    self.internal_read(
                        nametable_base +
                        coarse_y * 32 +
                        coarse_x,
                    );

                let attribute_addr =
                    nametable_base +
                    0x3c0 +
                    (coarse_y / 4) * 8 +
                    (coarse_x / 4);

                let attribute =
                    self.internal_read(
                        attribute_addr,
                    );

                let shift =
                    ((coarse_y % 4) / 2) * 4 +
                    ((coarse_x % 4) / 2) * 2;

                let palette =
                    (attribute >> shift) & 3;

                let p0 =
                    self.internal_read(
                        pattern +
                        tile as u16 * 16 +
                        fine_y,
                    );

                let p1 =
                    self.internal_read(
                        pattern +
                        tile as u16 * 16 +
                        fine_y +
                        8,
                    );

                for bit in 0..8 {
                    let px =
                        pixel_x + bit;

                    if px < 0 || px >= 256 {
                        continue;
                    }

                    let bit0 =
                        (p0 >> (7 - bit)) & 1;

                    let bit1 =
                        (p1 >> (7 - bit)) & 1;

                    let color_index =
                        (bit1 << 1) | bit0;

                    let px =
                        px as usize;

                    if color_index == 0 {
                        self.bg_opaque[px] = 0;

                        self.framebuffer[
                            y * WIDTH + px
                        ] = rgb(
                            backdrop.0,
                            backdrop.1,
                            backdrop.2,
                        );
                    } else {
                        self.bg_opaque[px] = 1;

                        let color =
                            NES_PALETTE[
                                (
                                    self.internal_read(
                                        0x3f00 +
                                        palette as u16 * 4 +
                                        color_index as u16,
                                    ) & 0x3f
                                ) as usize
                            ];

                        self.framebuffer[
                            y * WIDTH + px
                        ] = rgb(
                            color.0,
                            color.1,
                            color.2,
                        );
                    }
                }

                pixel_x += 8;

                if coarse_x == 31 {
                    v =
                        (v & !0x001f) ^
                        0x0400;
                } else {
                    v += 1;
                }
            }
        }

        if sprites_enabled {
            self.render_sprites(y);
        }

        if background_enabled {
            self.v =
                Self::inc_vertical(self.v);
        }
    }

    fn render_sprites(
        &mut self,
        y: usize,
    ) {
        let height =
            if self.ctrl & 0x20 != 0 {
                16
            } else {
                8
            };

        let mut visible = Vec::with_capacity(8);

        for index in 0..64 {
            let sprite_y =
                self.oam[index * 4] as i32;

            // Sprite data is delayed by one scanline: a sprite
            // whose OAM Y is N first appears on scanline N + 1
            // (which is why software writes Y - 1 to OAM). So the
            // sprite's first row is drawn when `y == sprite_y + 1`.
            let row =
                y as i32 - (sprite_y + 1);

            if row >= 0 &&
               row < height
            {
                // Only 8 sprites fit on a scanline. Finding a
                // *ninth* in range is what sets the overflow flag;
                // the flag must not be raised by a line that has
                // exactly eight sprites. (Real hardware's overflow
                // check is famously buggy and can false-positive /
                // false-negative depending on OAM contents; this is
                // the intended "more than 8 sprites" behaviour,
                // which is what well-behaved games expect.)
                if visible.len() == 8 {
                    self.status |= 0x20;
                    break;
                }

                visible.push(index);
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
        struct PreparedSprite {
            index: usize,
            sprite_x: i32,
            horizontal_flip: bool,
            behind_background: bool,
            palette: u8,
            p0: u8,
            p1: u8,
        }

        let mut prepared =
            Vec::with_capacity(visible.len());

        for &index in visible.iter() {
            let sprite_y =
                self.oam[index * 4] as i32;

            // Same one-scanline delay as in the range check above.
            let mut row =
                y as i32 - (sprite_y + 1);

            let tile =
                self.oam[index * 4 + 1];

            let attributes =
                self.oam[index * 4 + 2];

            let sprite_x =
                self.oam[index * 4 + 3] as i32;

            let vertical_flip =
                attributes & 0x80 != 0;

            let horizontal_flip =
                attributes & 0x40 != 0;

            let behind_background =
                attributes & 0x20 != 0;

            let palette =
                attributes & 3;

            if vertical_flip {
                row =
                    height - 1 - row;
            }

            let (pattern, tile_number, row) =
                if height == 16 {
                    let pattern =
                        if tile & 1 != 0 {
                            0x1000
                        } else {
                            0
                        };

                    let mut tile_number =
                        tile & 0xfe;

                    let mut row =
                        row;

                    if row >= 8 {
                        tile_number += 1;
                        row -= 8;
                    }

                    (
                        pattern,
                        tile_number,
                        row,
                    )
                } else {
                    (
                        if self.ctrl & 8 != 0 {
                            0x1000
                        } else {
                            0
                        },
                        tile,
                        row,
                    )
                };

            let p0 =
                self.internal_read(
                    pattern +
                    tile_number as u16 * 16 +
                    row as u16,
                );

            let p1 =
                self.internal_read(
                    pattern +
                    tile_number as u16 * 16 +
                    row as u16 +
                    8,
                );

            prepared.push(PreparedSprite {
                index,
                sprite_x,
                horizontal_flip,
                behind_background,
                palette,
                p0,
                p1,
            });
        }

        for sprite in prepared.iter().rev() {
            let index = sprite.index;
            let sprite_x = sprite.sprite_x;
            let horizontal_flip = sprite.horizontal_flip;
            let behind_background = sprite.behind_background;
            let palette = sprite.palette;
            let p0 = sprite.p0;
            let p1 = sprite.p1;

            for column in 0..8 {
                let bit =
                    if horizontal_flip {
                        column
                    } else {
                        7 - column
                    };

                let bit0 =
                    (p0 >> bit) & 1;

                let bit1 =
                    (p1 >> bit) & 1;

                let color_index =
                    (bit1 << 1) | bit0;

                if color_index == 0 {
                    continue;
                }

                let screen_x =
                    sprite_x +
                    column as i32;

                if !(0..256).contains(&screen_x) {
                    continue;
                }

                let screen_x =
                    screen_x as usize;

                if index == 0 &&
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

                if behind_background &&
                   self.bg_opaque[screen_x] != 0
                {
                    continue;
                }

                let color =
                    NES_PALETTE[
                        (
                            self.internal_read(
                                0x3f10 +
                                palette as u16 * 4 +
                                color_index as u16,
                            ) & 0x3f
                        ) as usize
                    ];

                self.framebuffer[
                    y * WIDTH + screen_x
                ] = rgb(
                    color.0,
                    color.1,
                    color.2,
                );
            }
        }
    }
}

fn rgb(
    r: u8,
    g: u8,
    b: u8,
) -> u32 {
    ((r as u32) << 16) |
    ((g as u32) << 8) |
    b as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::test_support::make_cart;

    const BACKDROP_RGB: u32 = 0x000000; // NES_PALETTE[$0F]
    const SPRITE_RGB: u32 = 0xecEEec; // NES_PALETTE[$30]

    /// PPU with sprites enabled, a solid opaque tile 0 in CHR RAM,
    /// a black backdrop and a white sprite colour.
    fn ppu_with_solid_sprite_tile() -> Ppu {
        let mut ppu = Ppu::new(make_cart(0, 2, 0, false));

        for row in 0..8u16 {
            ppu.internal_write(row, 0xff); // plane 0
            ppu.internal_write(row + 8, 0x00); // plane 1 -> colour 1
        }

        ppu.palette[0x00] = 0x0f; // backdrop: black
        ppu.palette[0x11] = 0x30; // sprite palette 0, colour 1: white
        ppu.mask = 0x10; // sprites on, background off

        ppu
    }

    fn set_sprite(ppu: &mut Ppu, index: usize, y: u8, x: u8) {
        ppu.oam[index * 4] = y;
        ppu.oam[index * 4 + 1] = 0; // tile 0
        ppu.oam[index * 4 + 2] = 0; // palette 0, in front
        ppu.oam[index * 4 + 3] = x;
    }

    fn hide_all_sprites(ppu: &mut Ppu) {
        for i in 0..64 {
            ppu.oam[i * 4] = 0xff;
        }
    }

    fn pixel(ppu: &Ppu, x: usize, y: usize) -> u32 {
        ppu.framebuffer[y * WIDTH + x] & 0x00ff_ffff
    }

    /// Fix 4: a sprite with OAM Y = N is first visible on scanline
    /// N + 1 and covers scanlines N + 1 ..= N + 8.
    #[test]
    fn sprite_is_drawn_one_scanline_below_its_oam_y() {
        let mut ppu = ppu_with_solid_sprite_tile();
        hide_all_sprites(&mut ppu);
        set_sprite(&mut ppu, 0, 20, 100);

        for y in 0..240 {
            ppu.render_scanline(y);
        }

        assert_eq!(pixel(&ppu, 100, 20), BACKDROP_RGB, "line N is empty");
        assert_eq!(pixel(&ppu, 100, 21), SPRITE_RGB, "first row at N+1");
        assert_eq!(pixel(&ppu, 100, 28), SPRITE_RGB, "last row at N+8");
        assert_eq!(pixel(&ppu, 100, 29), BACKDROP_RGB, "line N+9 is empty");
    }

    #[test]
    fn sprite_y_of_ff_is_never_visible() {
        let mut ppu = ppu_with_solid_sprite_tile();
        hide_all_sprites(&mut ppu);

        for y in 0..240 {
            ppu.render_scanline(y);
            assert_eq!(pixel(&ppu, 0, y), BACKDROP_RGB);
        }
        assert_eq!(ppu.status & 0x20, 0);
    }

    /// Fix 5: exactly eight sprites on a line must NOT set the
    /// overflow flag.
    #[test]
    fn eight_sprites_on_a_line_do_not_set_overflow() {
        let mut ppu = ppu_with_solid_sprite_tile();
        hide_all_sprites(&mut ppu);
        for i in 0..8 {
            set_sprite(&mut ppu, i, 49, (i * 10) as u8);
        }

        ppu.render_scanline(50);

        assert_eq!(ppu.status & 0x20, 0);
        // All eight are still drawn.
        for i in 0..8 {
            assert_eq!(pixel(&ppu, i * 10, 50), SPRITE_RGB);
        }
    }

    /// Fix 5: a ninth sprite on the line sets overflow, and only
    /// the first eight (by OAM order) are drawn.
    #[test]
    fn nine_sprites_on_a_line_set_overflow() {
        let mut ppu = ppu_with_solid_sprite_tile();
        hide_all_sprites(&mut ppu);
        for i in 0..9 {
            set_sprite(&mut ppu, i, 49, (i * 10) as u8);
        }

        ppu.render_scanline(50);

        assert_ne!(ppu.status & 0x20, 0);
        assert_eq!(pixel(&ppu, 70, 50), SPRITE_RGB, "8th sprite drawn");
        assert_eq!(pixel(&ppu, 80, 50), BACKDROP_RGB, "9th sprite dropped");
    }

    /// Sprites on different lines must not count toward overflow.
    #[test]
    fn sprites_on_different_lines_do_not_overflow() {
        let mut ppu = ppu_with_solid_sprite_tile();
        hide_all_sprites(&mut ppu);
        for i in 0..20 {
            // Spread over 20 distinct, non-overlapping lines.
            set_sprite(&mut ppu, i, (i * 9) as u8, (i * 4) as u8);
        }

        for y in 0..240 {
            ppu.render_scanline(y);
        }

        assert_eq!(ppu.status & 0x20, 0);
    }

    /// 8x16 sprites use the same +1 delay.
    #[test]
    fn tall_sprites_use_the_same_one_line_delay() {
        let mut ppu = ppu_with_solid_sprite_tile();
        // Tile 0 top half is rows 0-7 (solid, written above); make
        // the bottom half (tile 1) solid too.
        for row in 0..8u16 {
            ppu.internal_write(16 + row, 0xff);
        }
        ppu.ctrl |= 0x20; // 8x16 sprites
        hide_all_sprites(&mut ppu);
        set_sprite(&mut ppu, 0, 30, 60);

        for y in 0..240 {
            ppu.render_scanline(y);
        }

        assert_eq!(pixel(&ppu, 60, 30), BACKDROP_RGB);
        assert_eq!(pixel(&ppu, 60, 31), SPRITE_RGB);
        assert_eq!(pixel(&ppu, 60, 46), SPRITE_RGB, "16th row at N+16");
        assert_eq!(pixel(&ppu, 60, 47), BACKDROP_RGB);
    }
}
