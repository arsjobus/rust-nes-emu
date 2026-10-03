use super::{LENGTH_TABLE, NOISE_PERIODS};

pub(super) struct Noise {
    pub(super) enabled: bool,
    pub(super) length: u8,
    length_halt: bool,
    halt_pending: Option<bool>,
    skip_length_clock: bool,

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
            length_halt: false,
            halt_pending: None,
            skip_length_clock: false,

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
                self.halt_pending = Some(value & 0x20 != 0);
                self.constant_volume = value & 0x10 != 0;
                self.volume = value & 0x0f;
            }

            2 => {
                self.mode = value & 0x80 != 0;
                self.period_index = (value & 0x0f) as usize;
            }

            3 => {
                if self.enabled {
                    self.length = LENGTH_TABLE[(value >> 3) as usize];
                }

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

            let feedback = (self.shift & 1) ^ ((self.shift >> tap) & 1);

            self.shift >>= 1;
            self.shift |= feedback << 14;
        } else {
            self.timer -= 1;
        }
    }

    /// Applies a halt-flag write made on an earlier cycle. The flag
    /// changes one clock after the write, i.e. after the length
    /// clock of the cycle that follows the write.
    pub(super) fn apply_halt(&mut self) {
        if let Some(halt) = self.halt_pending.take() {
            self.length_halt = halt;
        }
    }

    /// Register write that knows whether the next APU cycle clocks
    /// the length counters (`half_next`). A length reload landing on
    /// that cycle is ignored if the counter was non-zero; if it was
    /// zero the reload happens but the clock does not decrement it.
    pub(super) fn write_timed(&mut self, reg: u16, value: u8, half_next: bool) {
        let old = self.length;
        self.write(reg, value);
        if reg == 3 && half_next {
            super::resolve_length_reload(&mut self.length, &mut self.skip_length_clock, old);
        }
    }

    pub(super) fn clock_length(&mut self) {
        if std::mem::take(&mut self.skip_length_clock) {
            return;
        }
        if !self.length_halt && self.length > 0 {
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
        if !self.enabled || self.length == 0 || self.shift & 1 != 0 {
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
#[path = "../../tests/apu/noise.rs"]
mod tests;
