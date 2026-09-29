pub trait PostProcessEffect {
    fn name(&self) -> &'static str;

    fn enabled(&self) -> bool;

    fn set_enabled(&mut self, enabled: bool);

    fn apply(&self, framebuffer: &mut [u32], width: usize, height: usize);
}
