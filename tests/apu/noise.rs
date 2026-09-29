use super::*;

/// Counts LFSR steps over `cycles` CPU cycles, clocking the
/// timer once per CPU cycle (as `Apu::step_cycle` now does).
fn lfsr_clocks(period_index: u8, cycles: u32) -> u32 {
    let mut noise = Noise::new();
    noise.write(2, period_index);

    let mut clocks = 0;
    let mut last = noise.shift;

    for _ in 0..cycles {
        noise.clock_timer();
        if noise.shift != last {
            clocks += 1;
            last = noise.shift;
        }
    }

    clocks
}

/// Fix 3: the LFSR must step exactly once per table period (in
/// CPU cycles). Previously every period came out doubled.
#[test]
fn shift_register_clocks_once_per_table_period() {
    // Rate 0: period 4 CPU cycles -> clocks at calls 1,5,..,397.
    assert_eq!(lfsr_clocks(0, 400), 100);

    // Rate 4: period 64 -> calls 1,65,..,1985 within 2000.
    assert_eq!(lfsr_clocks(4, 2000), 32);

    // Rate 15: period 4068 -> calls 1, 4069, 8137 within 12204.
    assert_eq!(lfsr_clocks(15, 4068 * 3), 3);
}

/// Every table entry must be honoured exactly.
#[test]
fn every_rate_matches_its_table_entry() {
    for (index, &period) in NOISE_PERIODS.iter().enumerate() {
        let period = period as u32;
        let cycles = period * 10;
        // First clock happens on call 1, then every `period`.
        let expected = (cycles - 1) / period + 1;

        assert_eq!(
            lfsr_clocks(index as u8, cycles),
            expected,
            "rate index {index} (period {period})"
        );
    }
}
