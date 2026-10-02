use super::test_support::make_cart;

/// Fix 1: reads below $8000 used to underflow and panic on
/// NROM, UxROM and GxROM.
#[test]
fn low_addresses_do_not_panic_on_any_mapper() {
    let carts = [
        make_cart(0, 2, 1, false),
        make_cart(2, 8, 0, false),
        make_cart(66, 8, 4, false),
        make_cart(7, 8, 1, false),
        make_cart(9, 8, 8, false),
        make_cart(4, 8, 8, false),
    ];

    for cart in carts.iter() {
        for addr in [0x4020u16, 0x5fff, 0x6000, 0x7fff] {
            let _ = cart.cpu_read(addr);
        }
    }
}

#[test]
fn axrom_switches_32k_prg_banks_and_single_screen_mirroring() {
    let mut cart = make_cart(7, 8, 1, true);

    // Each 32 KiB bank contains adjacent 16 KiB units.
    assert_eq!(cart.cpu_read(0x8000), 0);
    assert_eq!(cart.cpu_read(0xc000), 1);
    assert_eq!(cart.mirroring, super::Mirroring::OneScreenLower);

    cart.cpu_write(0x8000, 2);
    assert_eq!(cart.cpu_read(0x8000), 4);
    assert_eq!(cart.cpu_read(0xc000), 5);
    assert_eq!(cart.mirroring, super::Mirroring::OneScreenLower);

    cart.cpu_write(0xffff, 0x13); // bank 3, upper single-screen page
    assert_eq!(cart.cpu_read(0x8000), 6);
    assert_eq!(cart.cpu_read(0xc000), 7);
    assert_eq!(cart.mirroring, super::Mirroring::OneScreenUpper);
}

#[test]
fn mmc3_banks_prg_chr_and_generates_irq() {
    let mut cart = make_cart(4, 8, 4, true);
    // MMC3 starts with R6/R7 at zero and the last two slots fixed.
    assert_eq!(cart.cpu_read(0x8000), 0);
    assert_eq!(cart.cpu_read(0xa000), 0);
    assert_eq!(cart.cpu_read(0xc000), 7);
    assert_eq!(cart.cpu_read(0xe000), 7);

    cart.cpu_write(0x8000, 6);
    cart.cpu_write(0x8001, 3);
    assert_eq!(cart.cpu_read(0x8000), 1);
    cart.cpu_write(0x8000, 0x46); // PRG mode 1
    assert_eq!(cart.cpu_read(0x8000), 7);
    assert_eq!(cart.cpu_read(0xc000), 1);

    cart.cpu_write(0x8000, 2);
    cart.cpu_write(0x8001, 16);
    assert_eq!(cart.chr_read(0x1000), 2);
    cart.cpu_write(0x8000, 0x82); // invert CHR mapping
    assert_eq!(cart.chr_read(0x0000), 2);

    cart.cpu_write(0xc000, 2);
    cart.cpu_write(0xc001, 0);
    cart.cpu_write(0xe001, 0);
    cart.clock_scanline();
    assert!(!cart.irq_pending());
    cart.clock_scanline();
    cart.clock_scanline();
    assert!(cart.irq_pending());
    cart.cpu_write(0xe000, 0);
    assert!(!cart.irq_pending());
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

fn mmc1_write_register(cart: &mut super::Cartridge, addr: u16, value: u8) {
    for bit in 0..5 {
        cart.cpu_write(addr, (value >> bit) & 1);
    }
}

#[test]
fn mmc1_serial_writes_select_prg_banks_and_modes() {
    let mut cart = make_cart(1, 8, 1, true);

    // Default mode fixes the last bank at $C000 and switches $8000.
    mmc1_write_register(&mut cart, 0xe000, 2);
    assert_eq!(cart.cpu_read(0x8000), 2);
    assert_eq!(cart.cpu_read(0xc000), 7);

    // Mode 2 fixes bank 0 at $8000 and switches the upper bank.
    mmc1_write_register(&mut cart, 0x8000, 0x08);
    mmc1_write_register(&mut cart, 0xe000, 3);
    assert_eq!(cart.cpu_read(0x8000), 0);
    assert_eq!(cart.cpu_read(0xc000), 3);

    // Modes 0/1 switch an aligned 32 KiB pair; bit 0 is ignored.
    mmc1_write_register(&mut cart, 0x8000, 0x00);
    mmc1_write_register(&mut cart, 0xe000, 3);
    assert_eq!(cart.cpu_read(0x8000), 2);
    assert_eq!(cart.cpu_read(0xc000), 3);
}

#[test]
fn mmc1_reset_restores_switchable_first_prg_bank_mode() {
    let mut cart = make_cart(1, 8, 1, true);
    mmc1_write_register(&mut cart, 0xe000, 2);
    assert_eq!(cart.cpu_read(0x8000), 2);

    cart.cpu_write(0x8000, 0x80);
    // Reset forces mode 3 but preserves the selected PRG bank.
    assert_eq!(cart.cpu_read(0x8000), 2);
    assert_eq!(cart.cpu_read(0xc000), 7);

    // It also discards an incomplete serial value.
    cart.cpu_write(0xe000, 1);
    cart.cpu_write(0xe000, 0x80);
    for bit in 0..5 {
        cart.cpu_write(0xe000, (1 >> bit) & 1);
    }
    assert_eq!(cart.cpu_read(0x8000), 1);
}

#[test]
fn mmc1_selects_chr_in_8k_and_4k_modes() {
    let mut cart = make_cart(1, 2, 4, false);

    // In 8 KiB mode, bit 0 of CHR bank 0 is ignored.
    mmc1_write_register(&mut cart, 0xa000, 3);
    assert_eq!(cart.chr_read(0x0000), 1);
    assert_eq!(cart.chr_read(0x1000), 1);

    // In 4 KiB mode the two halves can select separate banks.
    mmc1_write_register(&mut cart, 0x8000, 0x1c);
    mmc1_write_register(&mut cart, 0xa000, 2);
    mmc1_write_register(&mut cart, 0xc000, 4);
    assert_eq!(cart.chr_read(0x0000), 1);
    assert_eq!(cart.chr_read(0x1000), 2);
}

#[test]
fn mmc1_mirroring_and_prg_ram_enable_follow_registers() {
    let mut cart = make_cart(1, 2, 1, false);

    mmc1_write_register(&mut cart, 0x8000, 0x0c); // one-screen lower
    assert_eq!(cart.mirroring, super::Mirroring::OneScreenLower);
    mmc1_write_register(&mut cart, 0x8000, 0x0d); // one-screen upper
    assert_eq!(cart.mirroring, super::Mirroring::OneScreenUpper);
    mmc1_write_register(&mut cart, 0x8000, 0x0e); // vertical
    assert_eq!(cart.mirroring, super::Mirroring::Vertical);
    mmc1_write_register(&mut cart, 0x8000, 0x0f); // horizontal
    assert_eq!(cart.mirroring, super::Mirroring::Horizontal);

    cart.cpu_write(0x6000, 0x5a);
    assert_eq!(cart.cpu_read(0x6000), 0x5a);
    mmc1_write_register(&mut cart, 0xe000, 0x10); // disable PRG RAM
    cart.cpu_write(0x6000, 0xa5);
    assert_eq!(cart.cpu_read(0x6000), 0);
    mmc1_write_register(&mut cart, 0xe000, 0);
    assert_eq!(cart.cpu_read(0x6000), 0x5a);
}

/// MMC3 boards power on with PRG RAM accessible; games (and test
/// ROMs) use $6000-$7FFF before ever writing $A001.
#[test]
fn mmc3_prg_ram_is_usable_at_power_on() {
    let mut cart = make_cart(4, 8, 8, false);

    cart.cpu_write(0x6000, 0x5a);
    assert_eq!(cart.cpu_read(0x6000), 0x5a);

    // ...and $A001 can still disable / write-protect it.
    cart.cpu_write(0xa001, 0xc0); // enabled, write-protected
    cart.cpu_write(0x6000, 0x11);
    assert_eq!(cart.cpu_read(0x6000), 0x5a);

    cart.cpu_write(0xa001, 0x00); // disabled
    assert_eq!(cart.cpu_read(0x6000), 0);
}

/// Writes to the unmapped $4020-$5FFF window must not reach mappers
/// that decode only $8000 and above.
#[test]
fn unmapped_writes_do_not_clock_mapper_registers() {
    // MMC1: five stray writes of 1 must not load the shift register.
    let mut mmc1 = make_cart(1, 8, 0, false);
    let before = mmc1.cpu_read(0x8000);
    for _ in 0..5 {
        mmc1.cpu_write(0x4020, 0x01);
        mmc1.cpu_write(0x5000, 0x01);
    }
    // Would have selected PRG bank 0b11111 = 15 -> bank 7 after modulo.
    assert_eq!(mmc1.cpu_read(0x8000), before);

    // GxROM / AxROM: a stray write must not switch banks.
    let mut gxrom = make_cart(66, 8, 4, false);
    let before = gxrom.cpu_read(0x8000);
    gxrom.cpu_write(0x5fff, 0x33);
    assert_eq!(gxrom.cpu_read(0x8000), before);

    let mut axrom = make_cart(7, 8, 1, false);
    axrom.cpu_write(0x5fff, 0x13);
    assert_eq!(axrom.cpu_read(0x8000), 0);
    assert_eq!(axrom.mirroring, super::Mirroring::OneScreenLower);
}

fn ines_image(mapper_low: u8, flags7: u8, tail: [u8; 4]) -> std::path::PathBuf {
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(b"NES\x1a");
    data[4] = 2;
    data[5] = 1;
    data[6] = mapper_low << 4;
    data[7] = flags7;
    data[12..16].copy_from_slice(&tail);
    data.extend(std::iter::repeat(0u8).take(2 * 0x4000 + 0x2000));

    let path = std::env::temp_dir().join(format!(
        "runes_header_{}_{}_{}.nes",
        std::process::id(),
        mapper_low,
        flags7
    ));
    std::fs::write(&path, &data).unwrap();
    path
}

/// Old dumps carry junk ("DiskDude!") in header bytes 7-15. That
/// used to turn mapper 1 into a bogus high-nibble mapper.
#[test]
fn dirty_ines_header_ignores_high_mapper_nibble() {
    let path = ines_image(1, 0x44, *b"Dude");
    let cart = super::Cartridge::load(path.to_str().unwrap());
    let _ = std::fs::remove_file(&path);
    assert!(matches!(
        cart.expect("loads as mapper 1").mapper_kind,
        super::MapperKind::Mmc1
    ));

    // A clean header still honours flags 7.
    let path = ines_image(1, 0x40, [0; 4]);
    let cart = super::Cartridge::load(path.to_str().unwrap());
    let _ = std::fs::remove_file(&path);
    assert!(cart.is_err(), "mapper 65 is unsupported");
}

#[test]
fn mmc1_ignores_the_second_write_of_a_read_modify_write() {
    let mut cart = make_cart(1, 8, 1, true);
    mmc1_write_register(&mut cart, 0xe000, 2);
    assert_eq!(cart.cpu_read(0x8000), 2);

    // First write of an RMW (the old value, bit 7 set) resets the shift
    // register and control; the second (new value) must be dropped.
    cart.cpu_write(0x8000, 0x80);
    cart.cpu_write_consecutive(0x8000, 0x01);
    // A clean 5-write sequence still works, proving the shift register
    // did not pick up the stray bit.
    mmc1_write_register(&mut cart, 0xe000, 5);
    assert_eq!(cart.cpu_read(0x8000), 5);

    // Other mappers accept the second write.
    let mut uxrom = make_cart(2, 4, 0, false);
    uxrom.cpu_write(0x8000, 1);
    uxrom.cpu_write_consecutive(0x8000, 2);
    assert_eq!(uxrom.cpu_read(0x8000), 2);
}

#[test]
fn mmc1_512k_surom_uses_chr_bit_4_as_outer_prg_bank() {
    let mut cart = make_cart(1, 32, 0, false); // 512 KiB PRG
    assert_eq!(cart.cpu_read(0xc000), 15); // last bank of the low half
    mmc1_write_register(&mut cart, 0xe000, 3);
    assert_eq!(cart.cpu_read(0x8000), 3);

    mmc1_write_register(&mut cart, 0xa000, 0x10); // outer bank -> upper half
    assert_eq!(cart.cpu_read(0x8000), 19);
    assert_eq!(cart.cpu_read(0xc000), 31);
}

#[test]
fn color_dreams_selects_prg_and_chr_from_one_register() {
    let mut cart = make_cart(11, 4, 4, false); // 2 x 32K PRG (16K units: 4)
    cart.cpu_write(0x8000, 0x21); // PRG 1, CHR 2
    assert_eq!(cart.cpu_read(0x8000), 2); // 32K bank 1 starts at 16K unit 2
    assert_eq!(cart.chr_read(0x0000), 2);
}

#[test]
fn nina03_registers_decode_in_the_4100_window() {
    let mut cart = make_cart(79, 4, 4, false);
    cart.cpu_write(0x4100, 0x0a); // PRG bank 1, CHR 2
    assert_eq!(cart.cpu_read(0x8000), 2);
    assert_eq!(cart.chr_read(0), 2);
    cart.cpu_write(0x4000, 0x00); // outside the window: ignored
    cart.cpu_write(0x4200, 0x00); // bit 8 clear: ignored
    assert_eq!(cart.chr_read(0), 2);
}

#[test]
fn camerica_fixes_last_bank_and_switches_first() {
    let mut cart = make_cart(71, 8, 0, false);
    assert_eq!(cart.cpu_read(0xc000), 7);
    cart.cpu_write(0xc000, 3);
    assert_eq!(cart.cpu_read(0x8000), 3);
    assert_eq!(cart.cpu_read(0xc000), 7);
    cart.cpu_write(0x9000, 0x10);
    assert_eq!(cart.mirroring, super::Mirroring::OneScreenUpper);
}

#[test]
fn uxrom_180_fixes_the_first_bank() {
    let mut cart = make_cart(180, 4, 0, false);
    cart.cpu_write(0x8000, 2);
    assert_eq!(cart.cpu_read(0x8000), 0);
    assert_eq!(cart.cpu_read(0xc000), 2);
}

#[test]
fn mapper_87_and_140_use_registers_in_the_prg_ram_window() {
    let mut cart = make_cart(87, 2, 4, false);
    cart.cpu_write(0x6000, 0x01); // bits swapped: CHR bank 2
    assert_eq!(cart.chr_read(0), 2);

    let mut cart = make_cart(140, 4, 4, false);
    cart.cpu_write(0x6000, 0x13); // PRG 1, CHR 3
    assert_eq!(cart.cpu_read(0x8000), 2);
    assert_eq!(cart.chr_read(0), 3);
}

#[test]
fn namco_108_has_no_irq_or_mirroring_registers() {
    let mut cart = make_cart(206, 8, 8, true);
    cart.cpu_write(0x8000, 6);
    cart.cpu_write(0x8001, 0x02);
    assert_eq!(cart.cpu_read(0x8000), 1); // 8K bank 2 lives in 16K unit 1
    cart.cpu_write(0xa000, 1); // MMC3 mirroring register: ignored
    assert!(cart.mirroring_vertical);
    cart.cpu_write(0xc000, 0);
    cart.cpu_write(0xc001, 0);
    cart.cpu_write(0xe001, 0);
    for _ in 0..4 {
        cart.clock_scanline();
    }
    assert!(!cart.irq_pending());
}

#[test]
fn txsrom_takes_nametable_page_from_chr_bank_bit_7() {
    let mut cart = make_cart(118, 8, 8, false);
    cart.cpu_write(0x8000, 0); // select R0
    cart.cpu_write(0x8001, 0x80); // bit 7 => page 1 for $2000/$2400
    assert_eq!(cart.nametable_ciram_page(0), Some(1));
    assert_eq!(cart.nametable_ciram_page(1), Some(1));
    assert_eq!(cart.nametable_ciram_page(2), Some(0));
}

fn mmc5_set(cart: &mut super::Cartridge, mode: u8, regs: [u8; 4]) {
    cart.cpu_write(0x5100, mode);
    for (i, reg) in regs.iter().enumerate() {
        cart.cpu_write(0x5114 + i as u16, *reg);
    }
}

/// 256 KiB PRG = 32 x 8 KiB banks; the helper fills each 16 KiB unit with
/// its own number, so 8 KiB bank n reads as n / 2.
fn mmc5_cart() -> super::Cartridge {
    make_cart(5, 16, 0, false)
}

#[test]
fn mmc5_prg_mode_0_maps_32k_from_5117_and_is_always_rom() {
    let mut cart = mmc5_cart();
    // Bit 7 clear on $5117 must NOT turn the window into RAM.
    mmc5_set(&mut cart, 0, [0, 0, 0, 0x0c]); // 32K bank 3 => 8K banks 12..15
    assert_eq!(cart.cpu_read(0x8000), 6);
    assert_eq!(cart.cpu_read(0xa000), 6);
    assert_eq!(cart.cpu_read(0xc000), 7);
    assert_eq!(cart.cpu_read(0xe000), 7);
}

#[test]
fn mmc5_prg_mode_1_maps_two_16k_halves() {
    let mut cart = mmc5_cart();
    mmc5_set(&mut cart, 1, [0, 0x84, 0, 0x0a]); // $8000: 8K bank 4 (ROM); $C000: 8K bank 10
    assert_eq!(cart.cpu_read(0x8000), 2);
    assert_eq!(cart.cpu_read(0xa000), 2);
    assert_eq!(cart.cpu_read(0xc000), 5);
    assert_eq!(cart.cpu_read(0xe000), 5);
}

#[test]
fn mmc5_prg_mode_2_maps_16k_then_two_8k() {
    let mut cart = mmc5_cart();
    mmc5_set(&mut cart, 2, [0, 0x84, 0x8b, 0x1f]); // 16K@4 ; $C000: bank 11 ; $E000: last
    assert_eq!(cart.cpu_read(0x8000), 2);
    assert_eq!(cart.cpu_read(0xa000), 2);
    assert_eq!(cart.cpu_read(0xc000), 5); // 8K bank 11 => unit 5
    assert_eq!(cart.cpu_read(0xe000), 15); // 8K bank 31 => unit 15, not bank 0
}

#[test]
fn mmc5_prg_mode_3_maps_four_8k_banks_and_5117_is_rom() {
    let mut cart = mmc5_cart();
    mmc5_set(&mut cart, 3, [0x82, 0x84, 0x86, 0x08]);
    assert_eq!(cart.cpu_read(0x8000), 1);
    assert_eq!(cart.cpu_read(0xa000), 2);
    assert_eq!(cart.cpu_read(0xc000), 3);
    assert_eq!(cart.cpu_read(0xe000), 4); // bit 7 clear on $5117 still ROM
}

/// 128 KiB CHR = 16 x 8 KiB; each 8 KiB unit is filled with its number.
fn mmc5_chr_cart(mode: u8) -> super::Cartridge {
    let mut cart = make_cart(5, 4, 16, false);
    cart.cpu_write(0x5101, mode);
    cart
}

#[test]
fn mmc5_chr_mode_0_uses_5127_for_the_sprite_set() {
    let mut cart = mmc5_chr_cart(0);
    cart.cpu_write(0x5127, 2); // 8 KiB unit 2
    assert_eq!(cart.chr_read(0x0000), 2);
    assert_eq!(cart.chr_read(0x1fff), 2);
}

#[test]
fn mmc5_chr_mode_1_sprite_set_uses_5123_and_5127() {
    let mut cart = mmc5_chr_cart(1);
    cart.cpu_write(0x5123, 2); // 4 KiB unit 2 => 8 KiB unit 1
    cart.cpu_write(0x5127, 6); // 4 KiB unit 6 => 8 KiB unit 3
    assert_eq!(cart.chr_read(0x0000), 1);
    assert_eq!(cart.chr_read(0x1000), 3);
}

/// Background fetch with 8x16 sprites enabled (separate register set).
fn bg(cart: &super::Cartridge, addr: u16) -> u8 {
    cart.bg_chr_read(addr, 0, 0, true, (0, 0))
}

#[test]
fn mmc5_background_set_in_4k_mode_uses_512b_for_both_tables() {
    let mut cart = mmc5_chr_cart(1);
    cart.cpu_write(0x512b, 6); // 4 KiB unit 6 => 8 KiB unit 3
    assert_eq!(bg(&cart, 0x0000), 3);
    assert_eq!(bg(&cart, 0x1000), 3);
}

#[test]
fn mmc5_background_set_in_2k_mode_repeats_across_both_tables() {
    let mut cart = mmc5_chr_cart(2);
    cart.cpu_write(0x5129, 4); // 2 KiB unit 4 => 8 KiB unit 1
    cart.cpu_write(0x512b, 8); // 2 KiB unit 8 => 8 KiB unit 2
    assert_eq!(bg(&cart, 0x0000), 1);
    assert_eq!(bg(&cart, 0x0800), 2);
    assert_eq!(bg(&cart, 0x1000), 1);
    assert_eq!(bg(&cart, 0x1800), 2);
}

#[test]
fn mmc5_background_set_in_8k_mode_uses_512b() {
    let mut cart = mmc5_chr_cart(0);
    cart.cpu_write(0x512b, 5);
    assert_eq!(bg(&cart, 0x0000), 5);
}

#[test]
fn mmc5_background_set_in_1k_mode_cycles_5128_to_512b() {
    let mut cart = mmc5_chr_cart(3);
    for (i, bank) in [0u8, 8, 16, 24].iter().enumerate() {
        cart.cpu_write(0x5128 + i as u16, *bank); // 1 KiB banks => 8 KiB units 0..3
    }
    assert_eq!(bg(&cart, 0x0000), 0);
    assert_eq!(bg(&cart, 0x0400), 1);
    assert_eq!(bg(&cart, 0x0800), 2);
    assert_eq!(bg(&cart, 0x0c00), 3);
    assert_eq!(bg(&cart, 0x1400), 1); // $1000 table repeats the same four
}

// ---- header handling -------------------------------------------------

use super::test_support::{load_image, raw_ines, unique_path};

#[test]
fn four_screen_flag_selects_four_screen_mirroring_and_survives_mapper_writes() {
    // Mapper 4 (flags6 high nibble) with the four-screen bit set, as on
    // DRROM-style boards. MMC3's $A000 mirroring register must not win.
    let mut cart = load_image(&raw_ines(2, 1, 0x48, 0)).unwrap();
    assert_eq!(cart.mirroring, super::Mirroring::FourScreen);

    cart.cpu_write(0xa000, 1);
    assert_eq!(cart.mirroring, super::Mirroring::FourScreen);
}

#[test]
fn nes2_exponent_notation_sizes_are_rejected() {
    let mut data = raw_ines(1, 1, 0, 0x08);
    data[9] = 0x0f; // PRG size nibble 0xF = exponent-multiplier form
    assert!(load_image(&data).is_err());
}

#[test]
fn trainer_is_loaded_at_7000() {
    let cart = load_image(&raw_ines(1, 1, 0x04, 0)).unwrap();
    assert_eq!(cart.cpu_read(0x7000), 1);
    assert_eq!(cart.cpu_read(0x7001), 2);
    assert_eq!(cart.cpu_read(0x71ff), ((511 % 251) + 1) as u8);
}

#[test]
fn battery_file_is_sized_from_the_header_not_the_64k_window() {
    // iNES 1.0 byte 8 = number of 8 KiB PRG RAM units.
    let mut data = raw_ines(1, 1, 0x02, 0);
    data[8] = 4;
    let path = unique_path();
    std::fs::write(&path, &data).unwrap();
    let cart = super::Cartridge::load(path.to_str().unwrap()).unwrap();
    cart.save_battery_ram().unwrap();
    let len = std::fs::metadata(path.with_extension("sav")).unwrap().len();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("sav"));
    assert_eq!(len, 4 * 8192);

    // No PRG RAM size in the header means the usual 8 KiB.
    let cart = load_image(&raw_ines(1, 1, 0x02, 0)).unwrap();
    assert_eq!(cart.save_size, 8192);
}

// ---- MMC5 ------------------------------------------------------------

#[test]
fn mmc5_status_register_reports_irq_pending_in_bit_7_and_in_frame_in_bit_6() {
    let mut cart = mmc5_cart();
    cart.cpu_write(0x5203, 2); // IRQ on scanline 2
    cart.cpu_write(0x5204, 0x80); // enable

    cart.clock_mmc5_scanline(0, true);
    assert_eq!(cart.cpu_read(0x5204), 0x40, "in frame, nothing pending yet");

    cart.clock_mmc5_scanline(1, true);
    cart.clock_mmc5_scanline(2, true);
    assert_eq!(
        cart.cpu_read(0x5204),
        0xc0,
        "pending (bit 7) + in frame (bit 6)"
    );
}

// ---- MMC3 index math -------------------------------------------------

/// The slot-by-slot index helpers must behave exactly like the old
/// table-building versions in every PRG/CHR mode.
#[test]
fn mmc3_prg_and_chr_banking_in_both_modes() {
    // 128 KiB PRG (16 x 8 KiB), 64 KiB CHR (64 x 1 KiB); each 16 KiB PRG unit
    // reads as its unit number, each 8 KiB CHR unit as its unit number.
    let mut cart = make_cart(4, 8, 8, false);
    let prg = |c: &super::Cartridge, a: u16| c.cpu_read(a);
    let reg = |c: &mut super::Cartridge, r: u8, v: u8, mode: u8| {
        c.cpu_write(0x8000, r | mode);
        c.cpu_write(0x8001, v);
    };

    // PRG mode 0: R6 @ $8000, R7 @ $A000, (-2) @ $C000, (-1) @ $E000.
    reg(&mut cart, 6, 2, 0); // 8K bank 2 => unit 1
    reg(&mut cart, 7, 5, 0); // 8K bank 5 => unit 2
    assert_eq!(prg(&cart, 0x8000), 1);
    assert_eq!(prg(&cart, 0xa000), 2);
    assert_eq!(prg(&cart, 0xc000), 7); // 8K bank 14 => unit 7
    assert_eq!(prg(&cart, 0xe000), 7); // 8K bank 15 => unit 7

    // PRG mode 1 swaps $8000 and $C000.
    reg(&mut cart, 6, 2, 0x40);
    assert_eq!(prg(&cart, 0x8000), 7);
    assert_eq!(prg(&cart, 0xc000), 1);
    assert_eq!(prg(&cart, 0xa000), 2);

    // CHR: R0 (2 KiB) @ $0000, R2 (1 KiB) @ $1000 without inversion...
    cart.cpu_write(0x8000, 0);
    cart.cpu_write(0x8001, 8);
    reg(&mut cart, 2, 17, 0);
    assert_eq!(cart.chr_read(0x0000), 1); // 1 KiB bank 8 => unit 1
    assert_eq!(cart.chr_read(0x0400), 1); // bank 9 => unit 1
    assert_eq!(cart.chr_read(0x1000), 2); // bank 17 => unit 2

    // ...and swapped with CHR inversion.
    cart.cpu_write(0x8000, 0x80);
    assert_eq!(cart.chr_read(0x1000), 1);
    assert_eq!(cart.chr_read(0x1400), 1);
    assert_eq!(cart.chr_read(0x0000), 2);
}
