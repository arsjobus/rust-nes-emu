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
