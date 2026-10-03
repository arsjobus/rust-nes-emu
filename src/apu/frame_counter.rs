use super::{noise::Noise, pulse::Pulse, triangle::Triangle};

/// APU frame sequencer.
///
/// `clock` is called once per CPU cycle. Step positions are in CPU
/// cycles after the sequencer was last restarted (NTSC):
///
///   4-step: 7457 (q)  14913 (q+h)  22371 (q)  29828 29829 (q+h) 29830 (IRQ, wrap)
///   5-step: 7457 (q)  14913 (q+h)  22371 (q)  37281 (q+h)       wrap at 37282
///
/// (q = envelope/linear-counter clock, h = length/sweep clock.) In
/// 4-step mode the frame IRQ flag is held set on three consecutive
/// cycles (29828-29830), so a read of $4015 on any of them sees it
/// and a read just before sees nothing.
///
/// A write to $4017 does not restart the sequencer at once: the
/// restart happens 3 CPU cycles later when the write landed on an even
/// cycle and 4 when it landed on an odd one. The IRQ-inhibit bit takes
/// effect immediately. Writing with bit 7 set clocks the quarter and
/// half frame units at the restart.
pub(super) struct FrameCounter {
    cycle: u32,
    mode_5: bool,
    irq_inhibit: bool,
    pub(super) irq_flag: bool,
    /// Last value written to $4017 (a warm reset rewrites it).
    last_value: u8,
    /// CPU cycles until a pending $4017 write restarts the sequencer
    /// (0 when none is pending).
    write_delay: u8,
    pending_mode_5: bool,
}

impl FrameCounter {
    /// The sequencer restarts a few cycles into the reset sequence, so
    /// it is already this many cycles into its count when the first
    /// instruction runs (hardware: 9-12 cycles, see apu_reset/4017_timing).
    const RESET_HEAD_START: u32 = 8;

    pub(super) fn new() -> Self {
        Self {
            cycle: 0,
            mode_5: false,
            irq_inhibit: false,
            irq_flag: false,
            last_value: 0,
            write_delay: 0,
            pending_mode_5: false,
        }
    }

    /// $4017 write, parity of the CPU cycle unknown (treated as even).
    #[cfg(test)]
    pub(super) fn write(&mut self, value: u8) {
        self.write_at(value, 0);
    }

    /// $4017 write that lands on CPU cycle `cpu_cycle`.
    pub(super) fn write_at(&mut self, value: u8, cpu_cycle: u64) {
        self.last_value = value;
        self.irq_inhibit = value & 0x40 != 0;
        if self.irq_inhibit {
            self.irq_flag = false;
        }
        self.pending_mode_5 = value & 0x80 != 0;
        self.write_delay = if cpu_cycle & 1 == 1 { 4 } else { 3 };
    }

    /// Power-on state: as if $4017 had been written with $00 just
    /// before the first instruction.
    pub(super) fn power_on() -> Self {
        let mut fc = Self::new();
        fc.cycle = Self::RESET_HEAD_START;
        fc
    }

    /// Warm reset: the IRQ flag is cleared and the last value written
    /// to $4017 is applied again.
    #[cfg(test)]
    pub(super) fn reset(&mut self) {
        self.irq_flag = false;
        self.irq_inhibit = self.last_value & 0x40 != 0;
        self.mode_5 = self.last_value & 0x80 != 0;
        self.write_delay = 0;
        self.cycle = Self::RESET_HEAD_START;
    }

    /// Whether the next call to `clock` will clock the length
    /// counters (a half-frame step).
    pub(super) fn half_frame_next(&self) -> bool {
        if self.write_delay > 0 {
            return self.write_delay == 1 && self.pending_mode_5;
        }
        let next = self.cycle + 1;
        match next {
            14913 => true,
            29829 => !self.mode_5,
            37281 => self.mode_5,
            _ => false,
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

        let mut restarted = false;
        if self.write_delay > 0 {
            self.write_delay -= 1;
            if self.write_delay == 0 {
                restarted = true;
                self.mode_5 = self.pending_mode_5;
                self.cycle = 0;
                if self.mode_5 {
                    quarter = true;
                    half = true;
                }
            }
        }

        if !restarted {
            self.cycle += 1;

            match self.cycle {
                7457 | 22371 => quarter = true,
                14913 => {
                    quarter = true;
                    half = true;
                }
                29828 if !self.mode_5 => {
                    if !self.irq_inhibit {
                        self.irq_flag = true;
                    }
                }
                29829 if !self.mode_5 => {
                    quarter = true;
                    half = true;
                    if !self.irq_inhibit {
                        self.irq_flag = true;
                    }
                }
                29830 if !self.mode_5 => {
                    if !self.irq_inhibit {
                        self.irq_flag = true;
                    }
                    self.cycle = 0;
                }
                37281 if self.mode_5 => {
                    quarter = true;
                    half = true;
                }
                37282 if self.mode_5 => self.cycle = 0,
                _ => {}
            }
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
