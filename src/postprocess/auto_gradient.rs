use super::effect::PostProcessEffect;

pub struct AutoGradient {
    enabled: bool,

    /*
     * Overall effect strength.
     *
     * 0.0 = original image
     * 1.0 = maximum effect
     */
    strength: f32,

    /*
     * How much the image changes from
     * top to bottom.
     */
    vertical: f32,

    /*
     * How much the edges differ from
     * the center.
     */
    horizontal: f32,
}

impl AutoGradient {
    pub fn new(strength: f32, vertical: f32, horizontal: f32) -> Self {
        Self {
            enabled: true,

            strength: strength.clamp(0.0, 1.0),

            vertical: vertical.clamp(0.0, 1.0),

            horizontal: horizontal.clamp(0.0, 1.0),
        }
    }
}

impl PostProcessEffect for AutoGradient {
    fn name(&self) -> &'static str {
        "auto_gradient"
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

        if width == 0 || height == 0 {
            return;
        }

        let pixel_count = width * height;

        if framebuffer.len() < pixel_count {
            return;
        }

        /*
         * -------------------------------------------------
         * STEP 1
         *
         * Calculate average scene brightness.
         * -------------------------------------------------
         */

        let mut brightness_sum = 0.0f32;

        for pixel in framebuffer.iter().take(pixel_count) {
            let r = ((pixel >> 16) & 0xff) as f32;

            let g = ((pixel >> 8) & 0xff) as f32;

            let b = (pixel & 0xff) as f32;

            /*
             * Perceived luminance.
             */
            let brightness = 0.2126 * r + 0.7152 * g + 0.0722 * b;

            brightness_sum += brightness;
        }

        let average_brightness = brightness_sum / pixel_count as f32;

        /*
         * 0.0 = very dark scene
         * 1.0 = very bright scene
         */
        let scene = (average_brightness / 255.0).clamp(0.0, 1.0);

        /*
         * Dark scenes receive a stronger
         * warm gradient.
         *
         * Bright scenes receive a weaker one.
         */
        let adaptive_strength = self.strength * (1.0 - scene * 0.45);

        /*
         * -------------------------------------------------
         * STEP 2
         *
         * Apply the gradient.
         * -------------------------------------------------
         */

        for y in 0..height {
            let y_normalized = y as f32 / (height - 1).max(1) as f32;

            /*
             * -1 at top
             *  0 at center
             * +1 at bottom
             */
            let vertical_position = y_normalized * 2.0 - 1.0;

            for x in 0..width {
                let x_normalized = x as f32 / (width - 1).max(1) as f32;

                /*
                 * -1 at left
                 *  0 at center
                 * +1 at right
                 */
                let horizontal_position = x_normalized * 2.0 - 1.0;

                /*
                 * -------------------------------------------------
                 * Vertical warm/cool gradient.
                 *
                 * Top:
                 *     slightly warmer/brighter
                 *
                 * Bottom:
                 *     slightly cooler/darker
                 * -------------------------------------------------
                 */

                let warm = -vertical_position * self.vertical * adaptive_strength;

                /*
                 * -------------------------------------------------
                 * Horizontal center glow.
                 *
                 * Center is brighter.
                 * Edges become darker.
                 * -------------------------------------------------
                 */

                let edge = horizontal_position.abs();

                let center_light = 1.0 - edge * self.horizontal * adaptive_strength;

                /*
                 * Overall brightness multiplier.
                 */
                let brightness_factor = (1.0 + warm * 0.30) * center_light;

                let index = y * width + x;

                let pixel = framebuffer[index];

                let r = ((pixel >> 16) & 0xff) as f32;

                let g = ((pixel >> 8) & 0xff) as f32;

                let b = (pixel & 0xff) as f32;

                /*
                 * Warm/cool color shift.
                 */
                let red = r * brightness_factor + warm * 18.0;

                let green = g * brightness_factor + warm * 5.0;

                let blue = b * brightness_factor - warm * 14.0;

                let red = red.clamp(0.0, 255.0) as u32;

                let green = green.clamp(0.0, 255.0) as u32;

                let blue = blue.clamp(0.0, 255.0) as u32;

                framebuffer[index] = (red << 16) | (green << 8) | blue;
            }
        }
    }
}
