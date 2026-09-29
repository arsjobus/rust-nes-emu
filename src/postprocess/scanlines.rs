use super::effect::PostProcessEffect;

pub struct Scanlines {
    enabled: bool,
    strength: f32,
}

impl Scanlines {
    pub fn new(strength: f32) -> Self {
        Self {
            enabled: true,
            strength: strength.clamp(0.0, 1.0),
        }
    }
}

impl PostProcessEffect for Scanlines {
    fn name(&self) -> &'static str {
        "scanlines"
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn apply(&self, framebuffer: &mut [u32], width: usize, height: usize) {
        for y in 0..height {
            // Darken alternate scanlines.
            if y % 2 == 0 {
                continue;
            }

            for x in 0..width {
                let index = y * width + x;

                let pixel = framebuffer[index];

                let r = ((pixel >> 16) & 0xff) as f32;
                let g = ((pixel >> 8) & 0xff) as f32;
                let b = (pixel & 0xff) as f32;

                let factor = 1.0 - self.strength;

                let r = (r * factor) as u32;
                let g = (g * factor) as u32;
                let b = (b * factor) as u32;

                framebuffer[index] = (r << 16) | (g << 8) | b;
            }
        }
    }
}
