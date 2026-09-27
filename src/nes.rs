use minifb::Key;

use crate::{
    bus::Bus,
    cartridge::Cartridge,
    cpu::Cpu,
    input::{Controller, NesButton},
    ppu::Ppu,
};

pub struct Nes {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl Nes {
    pub fn new(cart: Cartridge) -> Self {
        let controller = Controller::new();
        let ppu = Ppu::new(cart);

        let mut bus = Bus::new(
            ppu,
            controller,
        );

        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);

        Self {
            cpu,
            bus,
        }
    }

    pub fn run_frame(&mut self) {
        while !self.bus.ppu.frame_ready {
            let cycles =
                self.cpu.step(&mut self.bus);

            self.bus.ppu.catch_up(
                (cycles * 3) as i32
            );

            self.bus.apu.clock_cpu(cycles);

            if self.bus.ppu.nmi_pending
                && !self.bus.ppu.nmi_fired
            {
                self.bus.ppu.nmi_pending = false;
                self.bus.ppu.nmi_fired = true;

                self.cpu.nmi(&mut self.bus);
            }
        }

        self.bus.ppu.frame_ready = false;
    }

    pub fn framebuffer(&self) -> &[u32] {
        &self.bus.ppu.framebuffer
    }

    pub fn framebuffer_mut(&mut self) -> &mut [u32] {
        &mut self.bus.ppu.framebuffer
    }

    pub fn take_audio_samples(&mut self) -> Vec<f32> {
        self.bus.apu.take_samples()
    }

    pub fn update_input(
        &mut self,
        window: &minifb::Window,
    ) {
        /*
         * ---------------------------------------------------------
         * USB controller
         * ---------------------------------------------------------
         */
        self.bus.controller.update();

        /*
         * ---------------------------------------------------------
         * RockNES-style keyboard layout
         * ---------------------------------------------------------
         *
         * Arrow keys -> D-pad
         * Z          -> A
         * X          -> B
         * Space      -> Select
         * Enter      -> Start
         * ---------------------------------------------------------
         */

        self.bus.controller.set_button(
            NesButton::A,
            window.is_key_down(Key::Z),
        );

        self.bus.controller.set_button(
            NesButton::B,
            window.is_key_down(Key::X),
        );

        self.bus.controller.set_button(
            NesButton::Select,
            window.is_key_down(Key::Space),
        );

        self.bus.controller.set_button(
            NesButton::Start,
            window.is_key_down(Key::Enter),
        );

        self.bus.controller.set_button(
            NesButton::Up,
            window.is_key_down(Key::Up),
        );

        self.bus.controller.set_button(
            NesButton::Down,
            window.is_key_down(Key::Down),
        );

        self.bus.controller.set_button(
            NesButton::Left,
            window.is_key_down(Key::Left),
        );

        self.bus.controller.set_button(
            NesButton::Right,
            window.is_key_down(Key::Right),
        );
    }
}
