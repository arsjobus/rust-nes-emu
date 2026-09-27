use std::{
    thread,
    time::{
        Duration,
        Instant,
    },
};

use minifb::{
    Key,
    Window,
    WindowOptions,
};

use crate::{
    input::Controller,
    nes::Nes,
};

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

const FRAME_TIME: Duration =
    Duration::from_micros(16_667);

pub struct Video {
    window: Window,
}

impl Video {
    pub fn new() -> Self {
        let mut window =
            Window::new(
                "NES",
                WIDTH * 3,
                HEIGHT * 3,
                WindowOptions {
                    resize: false,
                    scale: minifb::Scale::X1,
                    ..WindowOptions::default()
                },
            )
            .expect(
                "Could not create window"
            );

        window.limit_update_rate(
            Some(FRAME_TIME)
        );

        Self {
            window,
        }
    }

    pub fn run(
        &mut self,
        nes: &mut Nes,
        max_frames: Option<u64>,
    ) {
        let mut frame_count = 0u64;

        let mut last =
            Instant::now();

        while self.window.is_open() &&
              !self.window.is_key_down(
                  Key::Escape
              )
        {
            nes.bus.controller.update(
                &self.window
            );

            nes.run_frame();

            frame_count += 1;

            self.window
                .update_with_buffer(
                    nes.framebuffer(),
                    WIDTH,
                    HEIGHT,
                )
                .expect(
                    "Failed to update window"
                );

            if let Some(max) =
                max_frames
            {
                if frame_count >= max {
                    break;
                }
            }

            let elapsed =
                last.elapsed();

            if elapsed < FRAME_TIME {
                thread::sleep(
                    FRAME_TIME - elapsed
                );
            }

            last = Instant::now();
        }
    }
}
