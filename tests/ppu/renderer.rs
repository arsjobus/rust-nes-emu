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
    ppu.mask = 0x14; // sprites on (including the left 8 pixels), background off

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
    assert_eq!(ppu.overflow_set_dot(50), None);
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

    // The flag is raised by sprite evaluation during the previous
    // line, so the renderer itself no longer touches it.
    assert!(ppu.overflow_set_dot(50).is_some());
    assert_eq!(pixel(&ppu, 70, 50), SPRITE_RGB, "8th sprite drawn");
    assert_eq!(pixel(&ppu, 80, 50), BACKDROP_RGB, "9th sprite dropped");
}

/// Hardware bug: after eight sprites are found, the PPU compares the
/// wrong OAM byte of later entries. A ninth in-range sprite whose Y
/// byte is not the one being compared is missed; a non-Y byte that
/// happens to look in range is a false positive.
#[test]
fn overflow_search_reproduces_the_diagonal_bug() {
    let mut ppu = ppu_with_solid_sprite_tile();
    hide_all_sprites(&mut ppu);
    for i in 0..8 {
        set_sprite(&mut ppu, i, 49, (i * 10) as u8);
    }
    // Entry 8 is compared on byte 0, entry 9 on byte 1 (the tile).
    // A genuinely in-range ninth sprite at entry 9 is missed ...
    set_sprite(&mut ppu, 9, 49, 0);
    assert_eq!(ppu.overflow_set_dot(50), None);
    // ... while a tile number that looks in range on entry 9 hits.
    ppu.oam[9 * 4] = 200;
    ppu.oam[9 * 4 + 1] = 49;
    assert!(ppu.overflow_set_dot(50).is_some());
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

/// $2001 bit 2 clear hides sprites in the leftmost 8 pixels.
#[test]
fn sprites_are_clipped_in_left_8_pixels_unless_enabled() {
    let mut ppu = ppu_with_solid_sprite_tile();
    hide_all_sprites(&mut ppu);
    set_sprite(&mut ppu, 0, 20, 4);

    ppu.mask = 0x10; // sprites on, left column masked
    ppu.render_scanline(21);
    assert_eq!(pixel(&ppu, 3, 21), BACKDROP_RGB);
    assert_eq!(pixel(&ppu, 7, 21), BACKDROP_RGB, "x < 8 is clipped");
    assert_eq!(pixel(&ppu, 8, 21), SPRITE_RGB, "x >= 8 is still drawn");

    ppu.mask = 0x14; // left column shown
    ppu.render_scanline(21);
    assert_eq!(pixel(&ppu, 4, 21), SPRITE_RGB);
}

/// $2001 bit 1 clear hides the background in the leftmost 8 pixels.
#[test]
fn background_is_clipped_in_left_8_pixels_unless_enabled() {
    let mut ppu = ppu_with_solid_sprite_tile();
    hide_all_sprites(&mut ppu);
    ppu.palette[0x01] = 0x30; // background palette 0, colour 1: white
                              // Nametable is all tile 0, which is solid colour 1.

    ppu.mask = 0x08; // background on, left column masked
    ppu.render_scanline(10);
    assert_eq!(pixel(&ppu, 0, 10), BACKDROP_RGB);
    assert_eq!(pixel(&ppu, 7, 10), BACKDROP_RGB);
    assert_eq!(pixel(&ppu, 8, 10), SPRITE_RGB);

    ppu.mask = 0x0a; // background on, left column shown
    ppu.render_scanline(10);
    assert_eq!(pixel(&ppu, 0, 10), SPRITE_RGB);
}

/// Vertical scroll advances whenever rendering is enabled, even
/// with sprites alone.
#[test]
fn sprite_only_rendering_still_advances_vertical_scroll() {
    let mut ppu = ppu_with_solid_sprite_tile();
    hide_all_sprites(&mut ppu);
    ppu.v = 0;

    ppu.render_scanline(0);

    assert_eq!(ppu.v & 0x7000, 0x1000, "fine Y incremented");
}

/// A "behind background" sprite that is hidden by an opaque background
/// pixel still owns that column: a higher-index sprite drawn there must
/// not show through it.
#[test]
fn hidden_behind_sprite_still_blocks_lower_priority_sprites() {
    let mut ppu = ppu_with_solid_sprite_tile();
    ppu.mask = 0x1e; // background + sprites, left 8 pixels shown
    ppu.palette[0x01] = 0x16; // background colour 1
    ppu.palette[0x15] = 0x2a; // sprite palette 1, colour 1
    hide_all_sprites(&mut ppu);

    set_sprite(&mut ppu, 0, 20, 100);
    ppu.oam[2] = 0x20; // sprite 0: behind background
    set_sprite(&mut ppu, 1, 20, 100);
    ppu.oam[6] = 0x01; // sprite 1: in front, palette 1

    ppu.render_scanline(24);

    let background = {
        let c = ppu.system_color(0x16);
        ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32
    };
    assert_eq!(pixel(&ppu, 102, 24), background);
}

/// Without a hiding background, the lowest-index sprite is on top.
#[test]
fn lowest_index_sprite_wins_over_overlapping_sprites() {
    let mut ppu = ppu_with_solid_sprite_tile();
    ppu.palette[0x15] = 0x2a;
    hide_all_sprites(&mut ppu);
    set_sprite(&mut ppu, 0, 20, 100);
    set_sprite(&mut ppu, 1, 20, 100);
    ppu.oam[6] = 0x01;

    ppu.render_scanline(24);

    assert_eq!(pixel(&ppu, 102, 24), SPRITE_RGB);
}
