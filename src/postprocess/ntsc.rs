use super::effect::PostProcessEffect;

pub struct Ntsc {
    enabled: bool,
    strength: f32,
    bleed: usize,
}

impl Ntsc {
    pub fn new(strength: f32, bleed: usize) -> Self {
        Self {
            enabled: true,
            strength: strength.clamp(0.0, 1.0),
            bleed: bleed.max(1),
        }
    }
}

impl PostProcessEffect for Ntsc {
    fn name(&self) -> &'static str {
        "ntsc"
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn apply(&self, framebuffer: &mut [u32], width: usize, height: usize) {
        if !self.enabled {
            return;
        }

        if framebuffer.len() < width * height {
            return;
        }

        /*
         * Keep the original frame.
         *
         * We must not sample pixels that have already
         * been modified by the NTSC filter.
         */
        let original = framebuffer.to_vec();

        /*
         * ---------------------------------------------------------
         * Convert the RGB framebuffer into a simplified YUV-like
         * representation.
         *
         * Y = luminance
         * U/V = chroma
         *
         * We keep luminance relatively sharp while allowing the
         * chroma channels to bleed horizontally.
         * ---------------------------------------------------------
         */

        let pixel_count = width * height;

        let mut y_channel = vec![0.0f32; pixel_count];

        let mut u_channel = vec![0.0f32; pixel_count];

        let mut v_channel = vec![0.0f32; pixel_count];

        for i in 0..pixel_count {
            let pixel = original[i];

            let r = ((pixel >> 16) & 0xff) as f32;

            let g = ((pixel >> 8) & 0xff) as f32;

            let b = (pixel & 0xff) as f32;

            /*
             * BT.601-style luminance.
             *
             * This is useful here because NTSC separates
             * brightness from color information.
             */
            let y = 0.299 * r + 0.587 * g + 0.114 * b;

            /*
             * Simplified chroma components.
             */
            let u = -0.14713 * r - 0.28886 * g + 0.436 * b;

            let v = 0.615 * r - 0.51499 * g - 0.10001 * b;

            y_channel[i] = y;
            u_channel[i] = u;
            v_channel[i] = v;
        }

        /*
         * ---------------------------------------------------------
         * Horizontal chroma blur.
         *
         * Composite video tends to smear color horizontally much
         * more than brightness.
         * ---------------------------------------------------------
         */

        let mut blurred_u = vec![0.0f32; pixel_count];

        let mut blurred_v = vec![0.0f32; pixel_count];

        let radius = self.bleed as i32;

        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;

                let mut sum_u = 0.0f32;

                let mut sum_v = 0.0f32;

                let mut weight_sum = 0.0f32;

                for offset in -radius..=radius {
                    let sample_x = x as i32 + offset;

                    if sample_x < 0 || sample_x >= width as i32 {
                        continue;
                    }

                    /*
                     * Center-weighted blur.
                     *
                     * Nearby pixels contribute more strongly.
                     */
                    let distance = offset.abs() as f32;

                    let weight = 1.0 - distance / (radius as f32 + 1.0);

                    let sample_index = y * width + sample_x as usize;

                    sum_u += u_channel[sample_index] * weight;

                    sum_v += v_channel[sample_index] * weight;

                    weight_sum += weight;
                }

                if weight_sum > 0.0 {
                    blurred_u[index] = sum_u / weight_sum;

                    blurred_v[index] = sum_v / weight_sum;
                }
            }
        }

        /*
         * ---------------------------------------------------------
         * Reconstruct RGB.
         *
         * Luminance comes mostly from the original pixel.
         * Chroma comes from the horizontally blurred signal.
         * ---------------------------------------------------------
         */

        for i in 0..pixel_count {
            let original_pixel = original[i];

            let original_r = ((original_pixel >> 16) & 0xff) as f32;

            let original_g = ((original_pixel >> 8) & 0xff) as f32;

            let original_b = (original_pixel & 0xff) as f32;

            /*
             * Original chroma.
             */
            let original_u = -0.14713 * original_r - 0.28886 * original_g + 0.436 * original_b;

            let original_v = 0.615 * original_r - 0.51499 * original_g - 0.10001 * original_b;

            /*
             * Blend original and blurred chroma according
             * to the requested effect strength.
             */
            let u = original_u + (blurred_u[i] - original_u) * self.strength;

            let v = original_v + (blurred_v[i] - original_v) * self.strength;

            let y = y_channel[i];

            /*
             * YUV -> RGB.
             */
            let r = y + 1.13983 * v;

            let g = y - 0.39465 * u - 0.58060 * v;

            let b = y + 2.03211 * u;

            let r = r.clamp(0.0, 255.0) as u32;

            let g = g.clamp(0.0, 255.0) as u32;

            let b = b.clamp(0.0, 255.0) as u32;

            framebuffer[i] = (r << 16) | (g << 8) | b;
        }
    }
}
