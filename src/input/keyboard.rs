use super::mapping::NesButtons;

/// Keyboard input state.
///
/// The actual keyboard event handling can be connected to whatever
/// windowing library your emulator uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardInput {
    buttons: NesButtons,
}

impl KeyboardInput {
    pub fn new() -> Self {
        Self {
            buttons: NesButtons::default(),
        }
    }

    pub fn buttons(&self) -> NesButtons {
        self.buttons
    }

    pub fn set_buttons(&mut self, buttons: NesButtons) {
        self.buttons = buttons;
    }
}
