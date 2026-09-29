use super::effect::PostProcessEffect;

pub struct PostProcessPipeline {
    effects: Vec<Box<dyn PostProcessEffect>>,
}

impl PostProcessPipeline {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn add<E>(&mut self, effect: E)
    where
        E: PostProcessEffect + 'static,
    {
        self.effects.push(Box::new(effect));
    }

    pub fn apply(&self, framebuffer: &mut [u32], width: usize, height: usize) {
        for effect in &self.effects {
            if effect.enabled() {
                effect.apply(framebuffer, width, height);
            }
        }
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(effect) = self.effects.iter_mut().find(|e| e.name() == name) {
            effect.set_enabled(enabled);
        }
    }

    pub fn cycle_lut(&mut self) {
        if let Some(effect) = self
            .effects
            .iter_mut()
            .find(|effect| effect.name() == "lut")
        {
            effect.cycle_lut();
        }
    }

    pub fn lut_label(&self) -> Option<&'static str> {
        self.effects
            .iter()
            .find(|effect| effect.name() == "lut")
            .and_then(|effect| effect.lut_label())
    }
}

impl Default for PostProcessPipeline {
    fn default() -> Self {
        Self::new()
    }
}
