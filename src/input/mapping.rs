use gilrs::Button;

#[derive(Debug, Clone, Copy, Default)]
pub struct NesButtons {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl NesButtons {
    pub fn set(&mut self, button: NesButton, pressed: bool) {
        match button {
            NesButton::A => self.a = pressed,
            NesButton::B => self.b = pressed,
            NesButton::Select => self.select = pressed,
            NesButton::Start => self.start = pressed,
            NesButton::Up => self.up = pressed,
            NesButton::Down => self.down = pressed,
            NesButton::Left => self.left = pressed,
            NesButton::Right => self.right = pressed,
        }
    }

    pub fn to_byte(self) -> u8 {
        let mut value = 0;

        if self.a {
            value |= 1 << 0;
        }

        if self.b {
            value |= 1 << 1;
        }

        if self.select {
            value |= 1 << 2;
        }

        if self.start {
            value |= 1 << 3;
        }

        if self.up {
            value |= 1 << 4;
        }

        if self.down {
            value |= 1 << 5;
        }

        if self.left {
            value |= 1 << 6;
        }

        if self.right {
            value |= 1 << 7;
        }

        value
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NesButton {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

/// Default Logitech/XInput-style controller mapping.
///
/// Controller:
///
///   South       -> NES A
///   East        -> NES B
///   Select      -> NES Select
///   Start       -> NES Start
///   D-pad       -> NES D-pad
///
/// The left analog stick is also supported by Controller::handle_axis().
pub fn map_button(button: Button) -> Option<NesButton> {
    match button {
        Button::South => Some(NesButton::A),
        Button::East => Some(NesButton::B),

        Button::Select => Some(NesButton::Select),
        Button::Start => Some(NesButton::Start),

        Button::DPadUp => Some(NesButton::Up),
        Button::DPadDown => Some(NesButton::Down),
        Button::DPadLeft => Some(NesButton::Left),
        Button::DPadRight => Some(NesButton::Right),

        _ => None,
    }
}
