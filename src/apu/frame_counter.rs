use super::{noise::Noise, pulse::Pulse, triangle::Triangle};

/// APU frame sequencer.
///
/// `clock` is called once per CPU cycle. Step positions are in CPU
/// cycles after the last $4017 write (NTSC):
///
///   4-step: 7457 (q)  14913 (q+h)  22371 (q)  29829 (q+h, IRQ)  wrap at 29830
///   5-step: 7457 (q)  14913 (q+h)  22371 (q)  37281 (q+h)       wrap at 37282
///
/// (q = envelope/linear-counter clock, h = length/sweep clock.)
pub(super) struct FrameCounter {
    cycle: u32,
    mode_5: bool,
    irq_inhibit: bool,
    pub(super) irq_flag: bool,
    /// A $4017 write with bit 7 set clocks the quarter and half
    /// frame units immediately.
    immediate_clock: bool,
}

impl FrameCounter {
    pub(super) fn new() -> Self {
        Self {
            cycle: 0,
            mode_5: false,
            irq_inhibit: false,
            irq_flag: false,
            immediate_clock: false,
        }
    }

    pub(super) fn write(&mut self, value: u8) {
        self.mode_5 = value & 0x80 != 0;
        self.irq_inhibit = value & 0x40 != 0;
        if self.irq_inhibit {
            self.irq_flag = false;
        }
        self.cycle = 0;
        if self.mode_5 {
            self.immediate_clock = true;
        }
    }

    pub(super) fn clock(
        &mut self,
        p1: &mut Pulse,
        p2: &mut Pulse,
        mmc5_p1: &mut Pulse,
        mmc5_p2: &mut Pulse,
        triangle: &mut Triangle,
        noise: &mut Noise,
    ) {
        let mut quarter = false;
        let mut half = false;

        if self.immediate_clock {
            self.immediate_clock = false;
            quarter = true;
            half = true;
        }

        self.cycle += 1;

        match self.cycle {
            7457 | 22371 => quarter = true,
            14913 => {
                quarter = true;
                half = true;
            }
            29829 if !self.mode_5 => {
                quarter = true;
                half = true;
                if !self.irq_inhibit {
                    self.irq_flag = true;
                }
            }
            29830 if !self.mode_5 => self.cycle = 0,
            37281 if self.mode_5 => {
                quarter = true;
                half = true;
            }
            37282 if self.mode_5 => self.cycle = 0,
            _ => {}
        }

        if quarter {
            p1.clock_envelope();
            p2.clock_envelope();
            mmc5_p1.clock_envelope();
            mmc5_p2.clock_envelope();
            noise.clock_envelope();

            triangle.clock_linear_counter();
        }

        if half {
            p1.clock_length();
            p2.clock_length();
            mmc5_p1.clock_length();
            mmc5_p2.clock_length();

            triangle.clock_length();
            noise.clock_length();

            p1.clock_sweep();
            p2.clock_sweep();
        }
    }
}

#[cfg(test)]
#[path = "../../tests/apu/frame_counter.rs"]
mod tests;
