use super::{LENGTH_TABLE, NOISE_PERIODS};

pub(super) struct Noise {
    pub(super) enabled: bool,
    pub(super) length: u8,

    mode: bool,
    period_index: usize,
    timer: u16,

    shift: u16,

    volume: u8,
    constant_volume: bool,

    envelope_start: bool,
    envelope_decay: u8,
    envelope_divider: u8,
    envelope_loop: bool,
}

impl Noise {
    pub(super) fn new() -> Self {
        Self {
            enabled: false,
            length: 0,

            mode: false,
            period_index: 0,
            timer: 0,

            shift: 1,

            volume: 0,
            constant_volume: false,

            envelope_start: false,
            envelope_decay: 0,
            envelope_divider: 0,
            envelope_loop: false,
        }
    }

    pub(super) fn write(&mut self, reg: u16, value: u8) {
        match reg {
            0 => {
                self.envelope_loop = value & 0x20 != 0;
                self.constant_volume = value & 0x10 != 0;
                self.volume = value & 0x0f;
            }

            2 => {
                self.mode = value & 0x80 != 0;
                self.period_index = (value & 0x0f) as usize;
            }

            3 => {
                self.length =
                    LENGTH_TABLE[(value >> 3) as usize];

                self.envelope_start = true;
            }

            _ => {}
        }
    }

    pub(super) fn clock_timer(&mut self) {
        if self.timer == 0 {
            // The timer counts (period - 1) .. 0 and then reloads,
            // so the interval between shift-register clocks is
            // exactly the table value (in CPU cycles).
            self.timer = NOISE_PERIODS[self.period_index] - 1;

            let tap = if self.mode { 6 } else { 1 };

            let feedback =
                (self.shift & 1)
                ^ ((self.shift >> tap) & 1);

            self.shift >>= 1;
            self.shift |= feedback << 14;
        } else {
            self.timer -= 1;
        }
    }

    pub(super) fn clock_length(&mut self) {
        if !self.envelope_loop && self.length > 0 {
            self.length -= 1;
        }
    }

    pub(super) fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_start = false;
            self.envelope_decay = 15;
            self.envelope_divider = self.volume;
            return;
        }

        if self.envelope_divider > 0 {
            self.envelope_divider -= 1;
        } else {
            self.envelope_divider = self.volume;

            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if self.envelope_loop {
                self.envelope_decay = 15;
            }
        }
    }

    pub(super) fn output(&self) -> f32 {
        if !self.enabled
            || self.length == 0
            || self.shift & 1 != 0
        {
            return 0.0;
        }

        if self.constant_volume {
            self.volume as f32
        } else {
            self.envelope_decay as f32
        }
    }
}


#[cfg(test)]
mod tests {
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
}
