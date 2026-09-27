pub(super) struct Dmc {
    pub(super) enabled: bool,
    pub(super) remaining: u16,
    pub(super) output: u8,
}

impl Dmc {
    pub(super) fn new() -> Self {
        Self {
            enabled: false,
            remaining: 0,
            output: 0,
        }
    }

    pub(super) fn write(&mut self, _reg: u16, _value: u8) {}
}
