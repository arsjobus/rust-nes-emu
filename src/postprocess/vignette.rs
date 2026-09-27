use super::effect::PostProcessEffect;

pub struct Vignette {
    enabled: bool,
    strength: f32,
}

impl Vignette {
    pub fn new(strength: f32) -> Self {
        Self {
            enabled: true,
            strength: strength.clamp(0.0, 1.0),
        }
    }
}

impl PostProcessEffect for Vignette {
    fn name(&self) -> &'static str {
        "vignette"
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn apply(
        &self,
        framebuffer: &mut [u32],
        width: usize,
        height: usize,
    ) {
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;

        for y in 0..height {
            for x in 0..width {
                let dx = (x as f32 - cx) / cx;
                let dy = (y as f32 - cy) / cy;

                let distance =
                    (dx * dx + dy * dy).sqrt();

                let factor =
                    1.0 - (distance * self.strength).min(1.0);

                let index = y * width + x;
                let pixel = framebuffer[index];

                let r = ((pixel >> 16) & 0xff) as f32;
                let g = ((pixel >> 8) & 0xff) as f32;
                let b = (pixel & 0xff) as f32;

                framebuffer[index] =
                    (((r * factor) as u32) << 16) |
                    (((g * factor) as u32) << 8) |
                    ((b * factor) as u32);
            }
        }
    }
}
