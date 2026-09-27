use super::effect::PostProcessEffect;

#[derive(Clone, Copy)]
pub enum LutPreset {
    Identity,
    WarmCrt,
    CoolCrt,
    Composite,
    GameBoy,
    Amber,
    HighContrast,
}

pub struct Lut {
    enabled: bool,
    preset: LutPreset,
    strength: f32,
}

impl Lut {
    pub fn new(
        preset: LutPreset,
        strength: f32,
    ) -> Self {
        Self {
            enabled: true,
            preset,
            strength: strength.clamp(0.0, 1.0),
        }
    }

    fn transform(
        &self,
        r: f32,
        g: f32,
        b: f32,
    ) -> (f32, f32, f32) {
        match self.preset {
            LutPreset::Identity => {
                (r, g, b)
            }

            /*
             * Warm CRT.
             */
            LutPreset::WarmCrt => {
                (
                    r * 1.05 + 0.015,
                    g * 1.01,
                    b * 0.92,
                )
            }

            /*
             * Cool CRT.
             */
            LutPreset::CoolCrt => {
                (
                    r * 0.94,
                    g * 0.99,
                    b * 1.06 + 0.01,
                )
            }

            /*
             * Composite-style color treatment.
             */
            LutPreset::Composite => {
                let y =
                    0.299 * r +
                    0.587 * g +
                    0.114 * b;

                let r2 =
                    y +
                    (r - y) * 0.82;

                let g2 =
                    y +
                    (g - y) * 0.88;

                let b2 =
                    y +
                    (b - y) * 0.72;

                (
                    r2 * 1.03,
                    g2 * 1.00,
                    b2 * 0.96,
                )
            }

            /*
             * Game Boy-style green palette.
             */
            LutPreset::GameBoy => {
                let luminance =
                    0.299 * r +
                    0.587 * g +
                    0.114 * b;

                /*
                 * Reduce luminance to four levels.
                 */
                let level =
                    (luminance * 3.0)
                        .round()
                        / 3.0;

                (
                    level * 0.35,
                    level * 0.72,
                    level * 0.42,
                )
            }

            /*
             * Amber monochrome display.
             */
            LutPreset::Amber => {
                let luminance =
                    0.299 * r +
                    0.587 * g +
                    0.114 * b;

                (
                    luminance * 0.95,
                    luminance * 0.58,
                    luminance * 0.16,
                )
            }

            /*
             * Stronger contrast.
             */
            LutPreset::HighContrast => {
                (
                    (r - 0.5) * 1.25 + 0.5,
                    (g - 0.5) * 1.25 + 0.5,
                    (b - 0.5) * 1.25 + 0.5,
                )
            }
        }
    }
}

impl PostProcessEffect for Lut {
    fn name(&self) -> &'static str {
        "lut"
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

            let r =
                ((pixel >> 16) & 0xff)
                    as f32 / 255.0;

            let g =
                ((pixel >> 8) & 0xff)
                    as f32 / 255.0;

            let b =
                (pixel & 0xff)
                    as f32 / 255.0;

            let (
                lut_r,
                lut_g,
                lut_b,
            ) =
                self.transform(
                    r,
                    g,
                    b,
                );

            /*
             * Blend LUT result with the original.
             *
             * 0.0 = original.
             * 1.0 = full LUT.
             */
            let final_r =
                r +
                (lut_r - r)
                    * self.strength;

            let final_g =
                g +
                (lut_g - g)
                    * self.strength;

            let final_b =
                b +
                (lut_b - b)
                    * self.strength;

            /*
             * Pack RGB back into 0xRRGGBB.
             *
             * Parentheses around the casts prevent
             * Rust from interpreting << as generic
             * arguments.
             */
            framebuffer[i] =
                (((final_r
                    .clamp(0.0, 1.0)
                    * 255.0) as u32) << 16)
                | (((final_g
                    .clamp(0.0, 1.0)
                    * 255.0) as u32) << 8)
                | ((final_b
                    .clamp(0.0, 1.0)
                    * 255.0) as u32);
        }
    }
}
