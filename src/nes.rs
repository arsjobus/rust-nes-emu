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

            self.bus.clock_apu(cycles);

            if self.bus.ppu.nmi_pending {
                self.bus.ppu.nmi_pending = false;

                self.bus.apu.debug_event("NMI");
                self.cpu.nmi(&mut self.bus);
            } else if self.bus.apu.irq_line() {
                // Level-triggered IRQ (currently the DMC's
                // sample-finished interrupt). Ignored while the
                // CPU's I flag is set; the game's handler is
                // expected to acknowledge it by writing $4015 or
                // $4010, which drops the line.
                self.cpu.irq(&mut self.bus);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::test_support::make_nrom_program;

    /// Hand-assembled program, loaded at $8000:
    ///
    ///   8000: LDA #$5A       ; PRG RAM round trip through the CPU
    ///   8002: STA $6000
    ///   8005: LDA $6000
    ///   8008: STA $00
    ///   800A: LDA #$80       ; enable NMI
    ///   800C: STA $2000
    ///   800F: JMP $800F      ; spin
    ///
    ///   8020: INC $01        ; NMI handler counts frames
    ///   8022: RTI
    fn program() -> Vec<u8> {
        let mut code = vec![
            0xa9, 0x5a,
            0x8d, 0x00, 0x60,
            0xad, 0x00, 0x60,
            0x85, 0x00,
            0xa9, 0x80,
            0x8d, 0x00, 0x20,
            0x4c, 0x0f, 0x80,
        ];
        code.resize(0x20, 0xea);
        code.extend_from_slice(&[0xe6, 0x01, 0x40]);
        code
    }

    #[test]
    fn runs_program_with_prg_ram_and_one_nmi_per_frame() {
        let cart = make_nrom_program(&program(), 0x8020, 0x8000, 0x8000);
        let mut nes = Nes::new(cart);

        // The first call only runs to the end of the power-on
        // pre-render line; after that each call is a full frame.
        for _ in 0..31 {
            nes.run_frame();
        }

        // Fix 1, through real CPU instructions: PRG RAM works and
        // nothing panicked reading/writing $6000.
        assert_eq!(nes.bus.ram[0], 0x5a);

        // Fix 2, end to end: exactly one NMI per vblank is
        // delivered now that `nmi_fired` is gone (30 full frames).
        let nmis = nes.bus.ram[1];
        assert!(
            (29..=30).contains(&nmis),
            "expected ~30 NMIs over 30 frames, got {nmis}"
        );
    }
}
