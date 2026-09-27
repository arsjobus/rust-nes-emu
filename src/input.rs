use minifb::{Key, Window};

pub struct Controller {
    state: u8,
    index: u8,
    strobe: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: 0,
            index: 0,
            strobe: false,
        }
    }

    pub fn update(&mut self, window: &Window) {
        let mut state = 0;

        if window.is_key_down(Key::Z) {
            state |= 1 << 0;
        }

        if window.is_key_down(Key::X) {
            state |= 1 << 1;
        }

        if window.is_key_down(Key::RightShift) {
            state |= 1 << 2;
        }

        if window.is_key_down(Key::Enter) {
            state |= 1 << 3;
        }

        if window.is_key_down(Key::Up) {
            state |= 1 << 4;
        }

        if window.is_key_down(Key::Down) {
            state |= 1 << 5;
        }

        if window.is_key_down(Key::Left) {
            state |= 1 << 6;
        }

        if window.is_key_down(Key::Right) {
            state |= 1 << 7;
        }

        self.state = state;
    }

    pub fn write(&mut self, value: u8) {
        self.strobe =
            value & 1 != 0;

        if self.strobe {
            self.index = 0;
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.strobe {
            return self.state & 1;
        }

        if self.index < 8 {
            let value =
                (self.state >> self.index) & 1;

            self.index += 1;

            value
        } else {
            1
        }
    }
}
