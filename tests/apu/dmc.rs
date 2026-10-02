use super::*;

const RATE_TABLE: [u32; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

/// Measures CPU cycles between output-level changes for a rate index,
/// clocking the timer once per APU cycle (every other CPU cycle).
fn cpu_cycles_per_bit(rate_index: u8) -> u32 {
    let mut dmc = Dmc::new();
    dmc.write(0, 0x40 | rate_index); // loop, IRQ off
    dmc.write(1, 0);
    dmc.write(2, 0);
    dmc.write(3, 0xff);
    dmc.set_enabled(true);

    let mut apu_cycles = 0u32;
    let mut last = dmc.output;
    let mut last_change = None;
    let mut intervals = Vec::new();

    while dmc.output < 120 && apu_cycles < 400_000 {
        dmc.clock_timer();
        apu_cycles += 1;
        if dmc.take_pending_fetch().is_some() {
            dmc.feed_byte(0xff);
        }
        if dmc.output != last {
            if let Some(prev) = last_change {
                intervals.push(apu_cycles - prev);
            }
            last_change = Some(apu_cycles);
            last = dmc.output;
        }
    }

    intervals.sort_unstable();
    intervals[intervals.len() / 2] * 2
}

/// The output level must change once per table period (CPU cycles). It
/// used to take `rate + 2`, making fast samples up to 3.7% flat.
#[test]
fn output_updates_once_per_rate_table_period() {
    for (index, expected) in RATE_TABLE.iter().enumerate() {
        assert_eq!(cpu_cycles_per_bit(index as u8), *expected, "rate {index}");
    }
}
