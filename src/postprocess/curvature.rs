use std::cell::RefCell;

use super::effect::PostProcessEffect;

pub struct Curvature {
    enabled: bool,

    /*
     * Curvature strength.
     *
     * 0.0 = completely flat
     * 0.05 = subtle
     * 0.10 = noticeable
     * 0.20 = strong
     */
    amount: f32,

    /*
     * Temporary framebuffer used during
     * the warp.
     */
    buffer: RefCell<Vec<u32>>,
}

impl Curvature {
    pub fn new(amount: f32) -> Self {
        Self {
            enabled: true,

            amount: amount.clamp(0.0, 0.5),

            buffer: RefCell::new(Vec::new()),
        }
    }
}

impl PostProcessEffect for Curvature {
    fn name(&self) -> &'static str {
        "curvature"
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

        let count = width * height;

        if framebuffer.len() < count {
            return;
        }

        let mut buffer = self.buffer.borrow_mut();

        if buffer.len() != count {
            buffer.resize(count, 0);
        }

        /*
         * Keep a copy of the original frame.
         */
        buffer[..count].copy_from_slice(&framebuffer[..count]);

        let source = &buffer[..count];

        /*
         * Normalize coordinates to:
         *
         * -1.0 .. +1.0
         *
         * with (0,0) at the center.
         */
        for y in 0..height {
            let ny = (y as f32 / (height - 1).max(1) as f32) * 2.0 - 1.0;

            for x in 0..width {
                let nx = (x as f32 / (width - 1).max(1) as f32) * 2.0 - 1.0;

                /*
                 * Radial distance from the center.
                 */
                let r2 = nx * nx + ny * ny;

                /*
                 * Barrel distortion.
                 *
                 * Positive amount produces the
                 * characteristic curved CRT look.
                 */
                let distortion = 1.0 + self.amount * r2;

                let source_x = nx * distortion;

                let source_y = ny * distortion;

                /*
                 * Outside the source image means
                 * we're looking at the black area
                 * around the curved screen.
                 */
                if source_x < -1.0 || source_x > 1.0 || source_y < -1.0 || source_y > 1.0 {
                    framebuffer[y * width + x] = 0;

                    continue;
                }

                /*
                 * Convert normalized coordinates
                 * back into framebuffer coordinates.
                 *
                 * Use nearest-neighbor sampling to
                 * preserve NES pixels.
                 */
                let sx = ((source_x + 1.0) * 0.5 * (width - 1) as f32).round() as usize;

                let sy = ((source_y + 1.0) * 0.5 * (height - 1) as f32).round() as usize;

                framebuffer[y * width + x] = source[sy * width + sx];
            }
        }
    }
}
