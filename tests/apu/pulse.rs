use super::*;

fn audible_pulse(channel_one: bool) -> Pulse {
    let mut p = Pulse::new(channel_one);
    p.enabled = true;
    p.write(0, 0xbf); // 50% duty, constant volume 15
    p.write(2, 0x00);
    p.write(3, 0x08); // period high bits 0, loads length
    p.sequence = 2; // a "high" step of the 50% duty cycle
    p
}

#[test]
fn length_loads_are_ignored_while_the_channel_is_disabled() {
    let mut p = Pulse::new(true);
    p.enabled = false;
    p.write(3, 0x08);
    assert_eq!(p.length, 0);

    p.enabled = true;
    p.write(3, 0x08);
    assert!(p.length > 0);
}

/// Hardware mutes the channel whenever the sweep *target* would exceed
/// $7FF - even with the sweep disabled - and never applies that update.
#[test]
fn sweep_target_overflow_mutes_without_changing_the_period() {
    let mut p = audible_pulse(true);
    p.write(2, 0x00);
    p.write(3, 0x0e); // period 0x600 (high bits 6)
    p.length = 10;
    p.sequence = 2;
    assert_eq!(p.period, 0x600);

    // Sweep disabled, shift 1: target = 0x600 + 0x300 = 0x900 > 0x7ff.
    p.write(1, 0x01);
    assert_eq!(p.output(), 0.0);

    // Enabled sweep must not rewrite the period either.
    p.write(1, 0x81);
    for _ in 0..4 {
        p.clock_sweep();
    }
    assert_eq!(p.period, 0x600);
    assert_eq!(p.output(), 0.0);
}

#[test]
fn sweep_still_adjusts_in_range_periods() {
    let mut p = audible_pulse(false);
    p.write(2, 0x00);
    p.write(3, 0x09); // period 0x100
    p.length = 10;
    p.write(1, 0x81); // enabled, period 0, shift 1, add
    p.clock_sweep(); // reload
    p.clock_sweep();
    assert_eq!(p.period, 0x100 + 0x80);
    assert!(p.output() > 0.0 || p.sequence == 0);
}
