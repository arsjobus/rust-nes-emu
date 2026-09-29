use super::effect::PostProcessEffect;

/// A lightweight CRT pass applied after nearest-neighbor upscaling.
/// Its three-pixel beam profile follows each source raster line at 3x scale.
pub struct Crt {
    enabled: bool,
    strength: f32,
}

impl Crt {
    pub fn new(strength: f32) -> Self {
        Self {
            enabled: false,
            strength: strength.clamp(0.0, 1.0),
        }
    }
}

impl PostProcessEffect for Crt {
    fn name(&self) -> &'static str {
        "crt"
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn apply(&self, framebuffer: &mut [u32], width: usize, height: usize) {
        let count = width.saturating_mul(height);
        if width == 0 || height == 0 || framebuffer.len() < count {
            return;
        }

        let strength = self.strength;
        let cx = (width.saturating_sub(1)) as f32 * 0.5;
        let cy = (height.saturating_sub(1)) as f32 * 0.5;
        let inv_cx = 1.0 / cx.max(1.0);
        let inv_cy = 1.0 / cy.max(1.0);

        for y in 0..height {
            // Each NES raster row occupies three output pixels. Keep the
            // center of the beam brighter and shade the gaps on either side.
            let scanline = match y % 3 {
                1 => 1.0,
                _ => 1.0 - 0.12 * strength,
            };
            for x in 0..width {
                let index = y * width + x;
                let pixel = framebuffer[index];
                let r = ((pixel >> 16) & 0xff) as f32;
                let g = ((pixel >> 8) & 0xff) as f32;
                let b = (pixel & 0xff) as f32;

                // Simulate the alternating red, green, and blue phosphors.
                let channel = x % 3;
                let dim = 1.0 - 0.035 * strength;
                let (r, g, b) = match channel {
                    0 => (r, g * dim, b * dim),
                    1 => (r * dim, g, b * dim),
                    _ => (r * dim, g * dim, b),
                };

                let nx = (x as f32 - cx) * inv_cx;
                let ny = (y as f32 - cy) * inv_cy;
                let edge = (1.0 - strength * 0.12 * (nx * nx + ny * ny)).clamp(0.0, 1.0);
                let factor = scanline * edge;
                framebuffer[index] = (((r * factor).round() as u32) << 16)
                    | (((g * factor).round() as u32) << 8)
                    | (b * factor).round() as u32;
            }
        }
    }
}
