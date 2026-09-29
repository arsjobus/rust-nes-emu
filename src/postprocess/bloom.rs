use super::effect::PostProcessEffect;

pub struct Bloom {
    enabled: bool,
    threshold: u8,
    strength: f32,
    radius: usize,
}

impl Bloom {
    pub fn new(threshold: u8, strength: f32, radius: usize) -> Self {
        Self {
            enabled: true,
            threshold,
            strength: strength.clamp(0.0, 1.0),
            radius: radius.max(1),
        }
    }
}

impl PostProcessEffect for Bloom {
    fn name(&self) -> &'static str {
        "bloom"
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
         * ---------------------------------------------------------
         * Step 1:
         *
         * Copy the original framebuffer.
         *
         * We need the original image because the bloom must not
         * repeatedly sample pixels that we have already modified.
         * ---------------------------------------------------------
         */
        let original = framebuffer.to_vec();

        /*
         * ---------------------------------------------------------
         * Step 2:
         *
         * Extract bright pixels.
         *
         * Only pixels brighter than `threshold` contribute to bloom.
         *
         * Instead of simply keeping the entire bright pixel, we
         * calculate how far above the threshold it is.
         *
         * This gives us a smoother transition.
         * ---------------------------------------------------------
         */
        let pixel_count = width * height;

        let mut bright_r = vec![0.0f32; pixel_count];

        let mut bright_g = vec![0.0f32; pixel_count];

        let mut bright_b = vec![0.0f32; pixel_count];

        for i in 0..pixel_count {
            let pixel = original[i];

            let r = ((pixel >> 16) & 0xff) as f32;

            let g = ((pixel >> 8) & 0xff) as f32;

            let b = (pixel & 0xff) as f32;

            /*
             * Perceived brightness.
             *
             * Green contributes more to perceived brightness,
             * followed by red, then blue.
             */
            let brightness = 0.2126 * r + 0.7152 * g + 0.0722 * b;

            if brightness <= self.threshold as f32 {
                continue;
            }

            /*
             * Convert brightness above the threshold
             * into a 0.0 -> 1.0 value.
             */
            let intensity = ((brightness - self.threshold as f32)
                / (255.0 - self.threshold as f32))
                .clamp(0.0, 1.0);

            bright_r[i] = r * intensity;

            bright_g[i] = g * intensity;

            bright_b[i] = b * intensity;
        }

        /*
         * ---------------------------------------------------------
         * Step 3:
         *
         * Blur horizontally.
         *
         * We use a simple weighted blur rather than a huge nested
         * kernel. This is much cheaper and works well at the NES's
         * tiny 256x240 resolution.
         * ---------------------------------------------------------
         */
        let mut horizontal_r = vec![0.0f32; pixel_count];

        let mut horizontal_g = vec![0.0f32; pixel_count];

        let mut horizontal_b = vec![0.0f32; pixel_count];

        let radius = self.radius as i32;

        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;

                let mut sum_r = 0.0f32;

                let mut sum_g = 0.0f32;

                let mut sum_b = 0.0f32;

                let mut weight_sum = 0.0f32;

                for offset in -radius..=radius {
                    let sample_x = x as i32 + offset;

                    if sample_x < 0 || sample_x >= width as i32 {
                        continue;
                    }

                    /*
                     * Simple Gaussian-like weighting.
                     *
                     * The center is strongest and the glow
                     * gradually becomes weaker toward the edge.
                     */
                    let distance = offset.abs() as f32;

                    let normalized = distance / (self.radius as f32 + 1.0);

                    let weight = 1.0 - normalized;

                    let sample_index = y * width + sample_x as usize;

                    sum_r += bright_r[sample_index] * weight;

                    sum_g += bright_g[sample_index] * weight;

                    sum_b += bright_b[sample_index] * weight;

                    weight_sum += weight;
                }

                if weight_sum > 0.0 {
                    horizontal_r[index] = sum_r / weight_sum;

                    horizontal_g[index] = sum_g / weight_sum;

                    horizontal_b[index] = sum_b / weight_sum;
                }
            }
        }

        /*
         * ---------------------------------------------------------
         * Step 4:
         *
         * Blur vertically.
         *
         * Doing horizontal + vertical passes gives us a
         * separable blur.
         * ---------------------------------------------------------
         */
        let mut bloom_r = vec![0.0f32; pixel_count];

        let mut bloom_g = vec![0.0f32; pixel_count];

        let mut bloom_b = vec![0.0f32; pixel_count];

        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;

                let mut sum_r = 0.0f32;

                let mut sum_g = 0.0f32;

                let mut sum_b = 0.0f32;

                let mut weight_sum = 0.0f32;

                for offset in -radius..=radius {
                    let sample_y = y as i32 + offset;

                    if sample_y < 0 || sample_y >= height as i32 {
                        continue;
                    }

                    let distance = offset.abs() as f32;

                    let normalized = distance / (self.radius as f32 + 1.0);

                    let weight = 1.0 - normalized;

                    let sample_index = sample_y as usize * width + x;

                    sum_r += horizontal_r[sample_index] * weight;

                    sum_g += horizontal_g[sample_index] * weight;

                    sum_b += horizontal_b[sample_index] * weight;

                    weight_sum += weight;
                }

                if weight_sum > 0.0 {
                    bloom_r[index] = sum_r / weight_sum;

                    bloom_g[index] = sum_g / weight_sum;

                    bloom_b[index] = sum_b / weight_sum;
                }
            }
        }

        /*
         * ---------------------------------------------------------
         * Step 5:
         *
         * Composite the blurred glow over the original image.
         * ---------------------------------------------------------
         */
        for i in 0..pixel_count {
            let pixel = original[i];

            let original_r = ((pixel >> 16) & 0xff) as f32;

            let original_g = ((pixel >> 8) & 0xff) as f32;

            let original_b = (pixel & 0xff) as f32;

            let final_r = original_r + bloom_r[i] * self.strength;

            let final_g = original_g + bloom_g[i] * self.strength;

            let final_b = original_b + bloom_b[i] * self.strength;

            framebuffer[i] = ((final_r.clamp(0.0, 255.0) as u32) << 16)
                | ((final_g.clamp(0.0, 255.0) as u32) << 8)
                | (final_b.clamp(0.0, 255.0) as u32);
        }
    }
}
