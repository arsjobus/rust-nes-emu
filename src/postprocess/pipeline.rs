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

    pub fn apply(
        &self,
        framebuffer: &mut [u32],
        width: usize,
        height: usize,
    ) {
        for effect in &self.effects {
            if effect.enabled() {
                effect.apply(
                    framebuffer,
                    width,
                    height,
                );
            }
        }
    }

    pub fn set_enabled(
        &mut self,
        name: &str,
        enabled: bool,
    ) {
        if let Some(effect) =
            self.effects.iter_mut()
                .find(|e| e.name() == name)
        {
            effect.set_enabled(enabled);
        }
    }

    pub fn toggle(&mut self, name: &str) {
        if let Some(effect) =
            self.effects.iter_mut()
                .find(|e| e.name() == name)
        {
            let enabled = effect.enabled();
            effect.set_enabled(!enabled);
        }
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.effects
            .iter()
            .find(|e| e.name() == name)
            .map(|e| e.enabled())
            .unwrap_or(false)
    }
}

impl Default for PostProcessPipeline {
    fn default() -> Self {
        Self::new()
    }
}
