use super::effect::PostProcessEffect;

pub struct ColorCorrection {
    enabled: bool,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    gamma: f32,
}

impl ColorCorrection {
    pub fn new(
        brightness: f32,
        contrast: f32,
        saturation: f32,
        gamma: f32,
    ) -> Self {
        Self {
            enabled: true,
            brightness,
            contrast: contrast.max(0.0),
            saturation: saturation.max(0.0),
            gamma: gamma.max(0.01),
        }
    }
}

impl PostProcessEffect for ColorCorrection {
    fn name(&self) -> &'static str {
        "color_correction"
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(
        &mut self,
        enabled: bool,
    ) {
        self.enabled = enabled;
    }

    fn apply(
        &self,
        framebuffer: &mut [u32],
        width: usize,
        height: usize,
    ) {
        if !self.enabled {
            return;
        }

        let count =
            width * height;

        if framebuffer.len() < count {
            return;
        }

        for i in 0..count {
            let pixel =
                framebuffer[i];

            let mut r =
                ((pixel >> 16) & 0xff)
                    as f32 / 255.0;

            let mut g =
                ((pixel >> 8) & 0xff)
                    as f32 / 255.0;

            let mut b =
                (pixel & 0xff)
                    as f32 / 255.0;

            /*
             * Gamma.
             *
             * 1.0 = unchanged.
             */
            r = r.powf(self.gamma);
            g = g.powf(self.gamma);
            b = b.powf(self.gamma);

            /*
             * Brightness.
             *
             * 0.0 = unchanged.
             */
            r += self.brightness;
            g += self.brightness;
            b += self.brightness;

            /*
             * Contrast.
             *
             * 1.0 = unchanged.
             */
            r =
                (r - 0.5)
                    * self.contrast
                    + 0.5;

            g =
                (g - 0.5)
                    * self.contrast
                    + 0.5;

            b =
                (b - 0.5)
                    * self.contrast
                    + 0.5;

            /*
             * Calculate luminance.
             */
            let luminance =
                0.2126 * r +
                0.7152 * g +
                0.0722 * b;

            /*
             * Saturation.
             *
             * 0.0 = grayscale.
             * 1.0 = unchanged.
             * >1.0 = more saturated.
             */
            r =
                luminance +
                (r - luminance)
                    * self.saturation;

            g =
                luminance +
                (g - luminance)
                    * self.saturation;

            b =
                luminance +
                (b - luminance)
                    * self.saturation;

            r =
                r.clamp(0.0, 1.0);

            g =
                g.clamp(0.0, 1.0);

            b =
                b.clamp(0.0, 1.0);

            /*
             * Pack RGB back into 0xRRGGBB.
             *
             * Parentheses around the casts are important:
             *
             * ((value as u32) << 16)
             */
            framebuffer[i] =
                (((r * 255.0) as u32) << 16)
                | (((g * 255.0) as u32) << 8)
                | ((b * 255.0) as u32);
        }
    }
}
