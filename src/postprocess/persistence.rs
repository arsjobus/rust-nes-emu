use std::cell::RefCell;

use super::effect::PostProcessEffect;

pub struct Persistence {
    enabled: bool,

    /*
     * Strength of the previous-frame contribution.
     *
     * 0.0 = disabled-looking
     * 0.10 = subtle
     * 0.20 = noticeable
     * 0.35 = strong
     * 0.50 = very strong
     */
    amount: f32,

    /*
     * Number of previous frames retained.
     */
    frames: usize,

    /*
     * Previous unmodified frames.
     *
     * history[0] = immediately previous frame
     * history[1] = two frames ago
     * history[2] = three frames ago
     */
    history: RefCell<Vec<Vec<u32>>>,

    initialized: RefCell<bool>,
}

impl Persistence {
    pub fn new(amount: f32, frames: usize) -> Self {
        Self {
            enabled: true,

            amount: amount.clamp(0.0, 0.95),

            frames: frames.max(1),

            history: RefCell::new(Vec::new()),

            initialized: RefCell::new(false),
        }
    }

    pub fn reset(&self) {
        self.history.borrow_mut().clear();

        *self.initialized.borrow_mut() = false;
    }
}

impl PostProcessEffect for Persistence {
    fn name(&self) -> &'static str {
        "persistence"
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        if self.enabled && !enabled {
            self.reset();
        }

        self.enabled = enabled;
    }

    fn apply(&self, framebuffer: &mut [u32], width: usize, height: usize) {
        if !self.enabled {
            return;
        }

        let count = width * height;

        if framebuffer.len() < count {
            return;
        }

        /*
         * Copy the original current frame BEFORE
         * modifying the framebuffer.
         */
        let current = framebuffer[..count].to_vec();

        let mut history = self.history.borrow_mut();

        /*
         * Reinitialize history if the framebuffer
         * dimensions changed.
         */
        let needs_reset =
            history.len() != self.frames || history.iter().any(|frame| frame.len() != count);

        if needs_reset {
            history.clear();

            for _ in 0..self.frames {
                history.push(current.clone());
            }

            *self.initialized.borrow_mut() = true;

            return;
        }

        /*
         * Weighted multi-frame accumulation.
         *
         * Example with amount = 0.35 and 3 history
         * frames:
         *
         * Current       65%
         * Previous      35% * 0.55
         * 2 frames ago  35% * 0.30
         * 3 frames ago  35% * 0.15
         *
         * The weights are normalized afterward.
         */

        let mut weights = Vec::with_capacity(self.frames + 1);

        weights.push(1.0);

        let mut previous_weight = self.amount;

        for _ in 0..self.frames {
            weights.push(previous_weight);

            previous_weight *= 0.55;
        }

        /*
         * Normalize all weights so the image doesn't
         * become brighter or darker.
         */
        let total_weight = weights.iter().sum::<f32>();

        for weight in weights.iter_mut() {
            *weight /= total_weight;
        }

        /*
         * Process every pixel.
         */
        for i in 0..count {
            let current_pixel = current[i];

            let mut r = ((current_pixel >> 16) & 0xff) as f32 * weights[0];

            let mut g = ((current_pixel >> 8) & 0xff) as f32 * weights[0];

            let mut b = (current_pixel & 0xff) as f32 * weights[0];

            /*
             * Add previous frames.
             */
            for frame_index in 0..self.frames {
                let pixel = history[frame_index][i];

                let weight = weights[frame_index + 1];

                r += ((pixel >> 16) & 0xff) as f32 * weight;

                g += ((pixel >> 8) & 0xff) as f32 * weight;

                b += (pixel & 0xff) as f32 * weight;
            }

            framebuffer[i] = (((r.clamp(0.0, 255.0)) as u32) << 16)
                | (((g.clamp(0.0, 255.0)) as u32) << 8)
                | ((b.clamp(0.0, 255.0)) as u32);
        }

        /*
         * Shift history backward.
         *
         * history[2] becomes history[3], etc.
         */
        for i in (1..self.frames).rev() {
            history[i] = history[i - 1].clone();
        }

        /*
         * Store the ORIGINAL current frame,
         * not the blurred result.
         */
        history[0] = current;
    }
}
