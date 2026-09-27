use super::{
    noise::Noise,
    pulse::Pulse,
    triangle::Triangle,
};

pub(super) struct FrameCounter {
    cycle: u32,
    mode_5: bool,
}

impl FrameCounter {
    pub(super) fn new() -> Self {
        Self {
            cycle: 0,
            mode_5: false,
        }
    }

    pub(super) fn write(&mut self, value: u8) {
        self.mode_5 = value & 0x80 != 0;
        self.cycle = 0;
    }

    pub(super) fn clock(
        &mut self,
        p1: &mut Pulse,
        p2: &mut Pulse,
        triangle: &mut Triangle,
        noise: &mut Noise,
    ) {
        self.cycle += 1;

        const QUARTER: u32 = 3729;

        if self.cycle % QUARTER == 0 {
            p1.clock_envelope();
            p2.clock_envelope();
            noise.clock_envelope();

            triangle.clock_linear_counter();
        }

        if self.cycle % (QUARTER * 2) == 0 {
            p1.clock_length();
            p2.clock_length();

            triangle.clock_length();
            noise.clock_length();

            p1.clock_sweep();
            p2.clock_sweep();
        }

        if !self.mode_5 && self.cycle >= QUARTER * 4 {
            self.cycle = 0;
        }

        if self.mode_5 && self.cycle >= QUARTER * 5 {
            self.cycle = 0;
        }
    }
}
