use super::*;
use crate::cartridge::test_support::make_cart;

fn ppu() -> Ppu {
    // CHR RAM cartridge so tests can write pattern data.
    Ppu::new(make_cart(0, 2, 0, false))
}

/// Fix 2: enabling NMI while the vblank flag is set must raise
/// an NMI immediately.
#[test]
fn enabling_nmi_during_vblank_raises_nmi() {
    let mut ppu = ppu();
    ppu.status |= 0x80;

    ppu.cpu_write(0, 0x80);

    assert!(ppu.nmi_pending);
}

#[test]
fn enabling_nmi_outside_vblank_does_not_raise_nmi() {
    let mut ppu = ppu();

    ppu.cpu_write(0, 0x80);

    assert!(!ppu.nmi_pending);
}

/// Only the 0 -> 1 transition counts; rewriting $2000 with the
/// bit already set must not raise a second NMI.
#[test]
fn rewriting_nmi_enable_does_not_retrigger() {
    let mut ppu = ppu();
    ppu.status |= 0x80;
    ppu.cpu_write(0, 0x80);
    ppu.nmi_pending = false; // CPU took it

    ppu.cpu_write(0, 0x80);

    assert!(!ppu.nmi_pending);
}

#[test]
fn disabling_nmi_cancels_pending_nmi() {
    let mut ppu = ppu();
    ppu.status |= 0x80;
    ppu.cpu_write(0, 0x80);
    assert!(ppu.nmi_pending);

    ppu.cpu_write(0, 0x00);

    assert!(!ppu.nmi_pending);
}

/// Reading $2002 clears the vblank flag, so enabling NMI
/// afterwards must not fire (the documented workaround).
#[test]
fn reading_status_first_prevents_immediate_nmi() {
    let mut ppu = ppu();
    ppu.status |= 0x80;

    let _ = ppu.cpu_read(2);
    ppu.cpu_write(0, 0x80);

    assert!(!ppu.nmi_pending);
}

/// The normal path: NMI enabled before vblank fires at the
/// start of vblank (scanline 241).
#[test]
fn nmi_fires_at_start_of_vblank_when_enabled() {
    let mut ppu = ppu();
    ppu.cpu_write(0, 0x80);
    assert!(!ppu.nmi_pending);

    // Power-on is scanline 261; 242 scanlines later we are on 241.
    ppu.catch_up(341 * 242);

    assert_eq!(ppu.scanline, 241);
    assert!(ppu.status & 0x80 != 0);
    assert!(ppu.nmi_pending);
}

#[test]
fn nmi_does_not_fire_at_vblank_when_disabled() {
    let mut ppu = ppu();

    ppu.catch_up(341 * 242);

    assert!(ppu.status & 0x80 != 0);
    assert!(!ppu.nmi_pending);
}

#[test]
fn mmc3_irq_clocks_near_end_of_visible_scanline() {
    let mut ppu = Ppu::new(make_cart(4, 8, 8, false));
    ppu.cart.cpu_write(0xc000, 1); // latch
    ppu.cart.cpu_write(0xc001, 0); // request reload
    ppu.cart.cpu_write(0xe001, 0); // enable
    ppu.mask = 0x18; // rendering enabled
    ppu.scanline = 0;

    ppu.catch_up(260);
    assert!(!ppu.cart.irq_pending());
    ppu.catch_up(1);
    assert!(!ppu.cart.irq_pending()); // first edge loads the latch
    ppu.catch_up(341 - 261 + 261);
    assert!(ppu.cart.irq_pending()); // next scanline edge decrements 1 -> 0
}

/// The pre-render line clocks the MMC3 counter too. Starting the
/// frame with a reload request and latch 1, the IRQ must assert after
/// the second clock (pre-render, then scanline 0).
#[test]
fn mmc3_irq_is_clocked_on_the_pre_render_line() {
    let mut ppu = Ppu::new(make_cart(4, 8, 8, false));
    ppu.cart.cpu_write(0xc000, 1); // latch
    ppu.cart.cpu_write(0xc001, 0); // request reload
    ppu.cart.cpu_write(0xe001, 0); // enable
    ppu.mask = 0x18;
    ppu.scanline = 261;
    ppu.dot = 0;

    ppu.catch_up(341); // whole pre-render line: reload to 1
    assert!(!ppu.cart.irq_pending());
    assert_eq!(ppu.scanline, 0);

    ppu.catch_up(341); // scanline 0: 1 -> 0
    assert!(ppu.cart.irq_pending());
}
