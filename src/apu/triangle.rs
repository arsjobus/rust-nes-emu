use super::LENGTH_TABLE;

pub(super) struct Triangle {
    pub(super) enabled: bool,

    timer: u16,
    period: u16,

    pub(super) length: u8,

    sequence: usize,

    linear_counter: u8,
    linear_reload_value: u8,
    linear_reload: bool,
    control_flag: bool,
}

impl Triangle {
    pub(super) fn new() -> Self {
        Self {
            enabled: false,

            timer: 0,
            period: 0,

            length: 0,

            sequence: 0,

            linear_counter: 0,
            linear_reload_value: 0,
            linear_reload: false,
            control_flag: false,
        }
    }

    pub(super) fn write(&mut self, reg: u16, value: u8) {
        match reg {
            0 => {
                self.control_flag = value & 0x80 != 0;
                self.linear_reload_value = value & 0x7f;
            }

            2 => {
                self.period = (self.period & 0x0700) | value as u16;
            }

            3 => {
                self.period = (self.period & 0x00ff) | (((value & 7) as u16) << 8);

                self.length = LENGTH_TABLE[(value >> 3) as usize];

                self.linear_reload = true;
            }

            _ => {}
        }
    }

    pub(super) fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.period;

            if self.length > 0 && self.linear_counter > 0 {
                self.sequence = (self.sequence + 1) & 31;
            }
        } else {
            self.timer -= 1;
        }
    }

    pub(super) fn clock_length(&mut self) {
        if !self.control_flag && self.length > 0 {
            self.length -= 1;
        }
    }

    pub(super) fn clock_linear_counter(&mut self) {
        if self.linear_reload {
            self.linear_counter = self.linear_reload_value;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        if !self.control_flag {
            self.linear_reload = false;
        }
    }

    pub(super) fn output(&self) -> f32 {
        if !self.enabled || self.length == 0 || self.linear_counter == 0 {
            return 0.0;
        }

        // Real hardware never silences the triangle channel itself -
        // many games (famously Mega Man 2) "mute" it by setting the
        // period to 0 or 1 instead of clearing the length counter,
        // which produces an ultrasonic tone that's naturally
        // filtered out by the analog output stage. A digital
        // emulator has no such filter, so faithfully reproducing
        // that ultrasonic wave aliases down into audible, buzzing
        // noise instead of the intended silence. This is a very
        // well-documented, common quirk (nesdev wiki: "these can be
        // eliminated in an emulator by halting the triangle channel
        // when an ultrasonic frequency is set (a timer value less
        // than 2)") - so we special-case it here, trading a small
        // amount of hardware "accuracy" (real hardware technically
        // isn't silent here either) for correct, listenable audio.
        if self.period < 2 {
            return 0.0;
        }

        let x = self.sequence as f32;

        if x < 16.0 { x } else { 31.0 - x }
    }
}
