use sdl2::keyboard::{KeyboardState, Scancode};

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

        let mut bus = Bus::new(ppu, controller);

        let mut cpu = Cpu::new();
        cpu.power_on(&mut bus);

        Self { cpu, bus }
    }

    /// Warm reset (the console's RESET button): CPU, APU and PPU
    /// registers are reset, RAM and cartridge state are kept.
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.bus.apu.reset();
        self.bus.ppu.reset();
        self.bus.reset_timing();
        self.cpu.reset(&mut self.bus);
        // The reset sequence itself lasts seven CPU cycles for the PPU
        // (the APU frame sequencer accounts for it in its head start).
        self.bus.ppu.catch_up(7 * 3);
    }

    pub fn save_battery_ram(&self) -> Result<(), String> {
        self.bus.ppu.cart.save_battery_ram()
    }

    pub fn run_frame(&mut self) {
        while !self.bus.ppu.frame_ready {
            let start_dot = self.bus.ppu.cycle;
            let mut cycles = self.cpu.step(&mut self.bus);
            let instruction_cycles = cycles;

            // OAM DMA halts the CPU for 513 cycles; during that time
            // the PPU and APU keep running.
            cycles += self.bus.take_dma_stall();

            // Dots already run inside the instruction (PPU register
            // reads) must not be counted twice.
            let already = self.bus.take_ppu_advanced();
            self.bus.ppu.catch_up((cycles * 3) as i32 - already);

            // Cycles already run inside the instruction (APU register
            // accesses) must not be counted twice either.
            let mut apu_done = self.bus.take_apu_advanced();

            // The CPU samples the IRQ line before the instruction's
            // final cycle, so a frame IRQ raised on that last cycle is
            // only taken after the next instruction. Run the APU up to
            // the poll point, sample, then finish the instruction.
            let poll_at = instruction_cycles.saturating_sub(1);
            if apu_done < poll_at {
                self.bus.clock_apu(poll_at - apu_done);
                apu_done = poll_at;
            }
            let apu_irq_polled = self.bus.apu.irq_line_polled();
            self.bus.clock_apu(cycles.saturating_sub(apu_done));

            let mut interrupt_cycles = 0;

            // The CPU polls for NMI one cycle before an instruction
            // ends; one raised later waits for the next instruction.
            // Instructions touching the PPU recorded their poll result
            // at that moment (a $2000/$2002 access may change it).
            let nmi_taken = self.bus.take_nmi_poll().unwrap_or_else(|| {
                let poll_dot = start_dot + (instruction_cycles.saturating_sub(1) * 3) as u64;
                self.bus.ppu.nmi_pending
                    && self.bus.ppu.nmi_raised_at + Ppu::NMI_POLL_LEAD <= poll_dot
            });

            if !nmi_taken && self.bus.ppu.nmi_pending && self.bus.ppu.nmi_delay {
                // NMI raised by enabling it in $2000 during vblank is
                // taken after the *next* instruction, not this one.
                self.bus.ppu.nmi_delay = false;
            } else if nmi_taken {
                self.bus.ppu.nmi_pending = false;
                self.bus.ppu.nmi_delay = false;

                self.bus.apu.debug_event("NMI");
                interrupt_cycles = self.cpu.nmi(&mut self.bus);
            } else if (apu_irq_polled || self.bus.ppu.cart.irq_pending())
                && self.cpu.irq(&mut self.bus)
            {
                // Level-triggered APU and mapper IRQ lines. Ignored while the
                // CPU's I flag is set; the game's handler is
                // expected to acknowledge it by writing $4015, $4010
                // or the mapper's IRQ acknowledge register.
                interrupt_cycles = 7;
            }

            if interrupt_cycles > 0 {
                self.bus.ppu.catch_up((interrupt_cycles * 3) as i32);

                self.bus.clock_apu(interrupt_cycles);
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

    pub fn turbo_a_enabled(&self) -> bool {
        self.bus.controller.turbo_a_enabled()
    }

    pub fn turbo_b_enabled(&self) -> bool {
        self.bus.controller.turbo_b_enabled()
    }

    pub fn take_audio_samples(&mut self) -> Vec<f32> {
        self.bus.apu.take_samples()
    }

    pub fn update_input(&mut self, keyboard: &KeyboardState<'_>) {
        /*
         * ---------------------------------------------------------
         * USB controller
         * ---------------------------------------------------------
         */
        self.bus.controller.update();
        self.bus
            .controller
            .set_keyboard_record_trigger(keyboard.is_scancode_pressed(Scancode::Num0));
        self.bus
            .controller
            .set_keyboard_turbo_trigger(keyboard.is_scancode_pressed(Scancode::LShift));

        /*
         * ---------------------------------------------------------
         * RockNES-style keyboard layout
         * ---------------------------------------------------------
         *
         * Arrow keys -> D-pad
         * X          -> A
         * Z          -> B
         * Space      -> Select
         * Enter      -> Start
         * ---------------------------------------------------------
         */

        self.bus
            .controller
            .set_button(NesButton::A, keyboard.is_scancode_pressed(Scancode::X));

        self.bus
            .controller
            .set_button(NesButton::B, keyboard.is_scancode_pressed(Scancode::Z));

        self.bus.controller.set_button(
            NesButton::Select,
            keyboard.is_scancode_pressed(Scancode::Space),
        );

        self.bus.controller.set_button(
            NesButton::Start,
            keyboard.is_scancode_pressed(Scancode::Return),
        );

        self.bus
            .controller
            .set_button(NesButton::Up, keyboard.is_scancode_pressed(Scancode::Up));

        self.bus.controller.set_button(
            NesButton::Down,
            keyboard.is_scancode_pressed(Scancode::Down),
        );

        self.bus.controller.set_button(
            NesButton::Left,
            keyboard.is_scancode_pressed(Scancode::Left),
        );

        self.bus.controller.set_button(
            NesButton::Right,
            keyboard.is_scancode_pressed(Scancode::Right),
        );
    }
}

#[cfg(test)]
#[path = "../tests/nes/mod.rs"]
mod tests;
