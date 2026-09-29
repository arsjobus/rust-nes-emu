use super::{LENGTH_TABLE, PULSE_DUTY};

pub(super) struct Pulse {
    pub(super) enabled: bool,

    duty: usize,
    sequence: usize,

    timer: u16,
    period: u16,

    pub(super) length: u8,

    volume: u8,
    constant_volume: bool,

    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
    envelope_loop: bool,

    sweep_enabled: bool,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_divider: u8,
    sweep_reload: bool,
    sweep_period: u8,

    channel_one: bool,
    mmc5: bool,
}

impl Pulse {
    pub(super) fn new(channel_one: bool) -> Self {
        Self {
            enabled: false,

            duty: 0,
            sequence: 0,

            timer: 0,
            period: 0,

            length: 0,

            volume: 0,
            constant_volume: false,

            envelope_start: false,
            envelope_divider: 0,
            envelope_decay: 0,
            envelope_loop: false,

            sweep_enabled: false,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_divider: 0,
            sweep_reload: false,
            sweep_period: 0,

            channel_one,
            mmc5: false,
        }
    }

    pub(super) fn new_mmc5() -> Self {
        let mut pulse = Self::new(false);
        pulse.mmc5 = true;
        pulse
    }

    pub(super) fn write(&mut self, reg: u16, value: u8) {
        match reg {
            0 => {
                self.duty = ((value >> 6) & 3) as usize;
                self.envelope_loop = value & 0x20 != 0;
                self.constant_volume = value & 0x10 != 0;
                self.volume = value & 0x0f;
            }

            1 => {
                if self.mmc5 {
                    return;
                }
                self.sweep_enabled = value & 0x80 != 0;
                self.sweep_period = ((value >> 4) & 7) + 1;
                self.sweep_negate = value & 0x08 != 0;
                self.sweep_shift = value & 7;
                self.sweep_reload = true;
            }

            2 => {
                self.period = (self.period & 0x0700) | value as u16;
            }

            3 => {
                self.period = (self.period & 0x00ff) | (((value & 7) as u16) << 8);

                self.length = LENGTH_TABLE[(value >> 3) as usize];

                self.sequence = 0;
                self.envelope_start = true;
            }

            _ => {}
        }
    }

    pub(super) fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.period;
            self.sequence = (self.sequence + 1) & 7;
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

    pub(super) fn clock_sweep(&mut self) {
        if self.sweep_divider == 0 {
            if self.sweep_enabled && self.sweep_shift != 0 {
                let change = self.period >> self.sweep_shift;

                if self.sweep_negate {
                    self.period = self
                        .period
                        .wrapping_sub(change)
                        .wrapping_sub(if self.channel_one { 1 } else { 0 });
                } else {
                    self.period = self.period.wrapping_add(change);
                }
            }

            self.sweep_divider = self.sweep_period;
        } else {
            self.sweep_divider -= 1;
        }

        if self.sweep_reload {
            self.sweep_reload = false;
            self.sweep_divider = self.sweep_period;
        }
    }

    pub(super) fn output(&self) -> f32 {
        if !self.enabled
            || self.length == 0
            || (!self.mmc5 && self.period < 8)
            || self.period > 0x7ff
        {
            return 0.0;
        }

        let duty = PULSE_DUTY[self.duty][self.sequence];

        if duty == 0.0 {
            return 0.0;
        }

        let volume = if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        };

        duty * volume as f32
    }

    pub(super) fn mmc5_output(&self) -> f32 {
        if !self.mmc5 {
            return self.output();
        }
        if !self.enabled || self.length == 0 || self.period > 0x7ff {
            return 0.0;
        }
        let duty = PULSE_DUTY[self.duty][self.sequence];
        let volume = if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        };
        (1.0 - duty) * volume as f32
    }
}
