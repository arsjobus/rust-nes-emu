use crate::{
    bus::Bus,
    cartridge::Cartridge,
    cpu::Cpu,
    input::Controller,
    ppu::Ppu,
};

pub struct Nes {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl Nes {
    pub fn new(
        cartridge: Cartridge,
        controller: Controller,
    ) -> Self {
        let ppu =
            Ppu::new(cartridge);

        let bus =
            Bus::new(
                ppu,
                controller,
            );

        Self {
            cpu: Cpu::new(),
            bus,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset(
            &mut self.bus
        );
    }

    pub fn run_frame(&mut self) {
        while !self.bus.ppu.frame_ready {
            let cycles =
                self.cpu.step(
                    &mut self.bus
                );

            self.bus.ppu.catch_up(
                (cycles * 3) as i32
            );

            if self.bus.ppu.nmi_pending &&
               !self.bus.ppu.nmi_fired
            {
                self.bus.ppu.nmi_pending =
                    false;

                self.bus.ppu.nmi_fired =
                    true;

                self.cpu.nmi(
                    &mut self.bus
                );
            }
        }

        self.bus.ppu.frame_ready =
            false;
    }

    pub fn framebuffer(&self) -> &[u32] {
        &self.bus.ppu.framebuffer
    }
}
