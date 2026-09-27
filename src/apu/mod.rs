mod dmc;
mod frame_counter;
mod noise;
mod pulse;
mod triangle;

use dmc::Dmc;
use frame_counter::FrameCounter;
use noise::Noise;
use pulse::Pulse;
use triangle::Triangle;

const CPU_CLOCK: f64 = 1_789_773.0;
const SAMPLE_RATE: f64 = 48_000.0;

pub const PULSE_DUTY: [[f32; 8]; 4] = [
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0],
];

pub const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2,
    40, 4, 80, 6,
    160, 8, 60, 10,
    14, 12, 26, 14,
    12, 16, 24, 18,
    48, 20, 96, 22,
    192, 24, 72, 26,
    16, 28, 32, 30,
];

pub const NOISE_PERIODS: [u16; 16] = [
    4, 8, 16, 32,
    64, 96, 128, 160,
    202, 254, 380, 508,
    762, 1016, 2034, 4068,
];

pub struct Apu {
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,

    frame_counter: FrameCounter,

    cpu_cycle: u64,

    sample_clock: f64,
    samples: Vec<f32>,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            pulse1: Pulse::new(true),
            pulse2: Pulse::new(false),
            triangle: Triangle::new(),
            noise: Noise::new(),
            dmc: Dmc::new(),

            frame_counter: FrameCounter::new(),

            cpu_cycle: 0,
            sample_clock: 0.0,
            samples: Vec::with_capacity(2048),
        }
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => self.status(),
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000..=0x4003 => {
                self.pulse1.write(addr - 0x4000, value);
            }
            0x4004..=0x4007 => {
                self.pulse2.write(addr - 0x4004, value);
            }
            0x4008..=0x400b => {
                self.triangle.write(addr - 0x4008, value);
            }
            0x400c..=0x400f => {
                self.noise.write(addr - 0x400c, value);
            }
            0x4010..=0x4013 => {
                self.dmc.write(addr - 0x4010, value);
            }
            0x4015 => self.set_enable(value),
            0x4017 => self.frame_counter.write(value),
            _ => {}
        }
    }

    pub fn clock_cpu(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.clock_one_cpu_cycle();
        }
    }

    fn clock_one_cpu_cycle(&mut self) {
        self.cpu_cycle += 1;

        self.frame_counter.clock(
            &mut self.pulse1,
            &mut self.pulse2,
            &mut self.triangle,
            &mut self.noise,
        );

        if self.cpu_cycle & 1 == 0 {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            self.noise.clock_timer();
        }

        self.triangle.clock_timer();

        self.sample_clock += SAMPLE_RATE / CPU_CLOCK;

        while self.sample_clock >= 1.0 {
            self.sample_clock -= 1.0;
            self.samples.push(self.mix());
        }
    }

    fn mix(&self) -> f32 {
        let p1 = self.pulse1.output();
        let p2 = self.pulse2.output();

        let pulse = p1 + p2;

        let pulse_out = if pulse <= 0.0 {
            0.0
        } else {
            95.88 / ((8128.0 / pulse) + 100.0)
        };

        let triangle = self.triangle.output();
        let noise = self.noise.output();
        let dmc = self.dmc.output as f32;

        let tnd =
            triangle / 8227.0 +
            noise / 12241.0 +
            dmc / 22638.0;

        let tnd_out = if tnd <= 0.0 {
            0.0
        } else {
            159.79 / ((1.0 / tnd) + 100.0)
        };

        (pulse_out + tnd_out).clamp(0.0, 1.0)
    }

    fn set_enable(&mut self, value: u8) {
        self.pulse1.enabled = value & 0x01 != 0;
        self.pulse2.enabled = value & 0x02 != 0;
        self.triangle.enabled = value & 0x04 != 0;
        self.noise.enabled = value & 0x08 != 0;
        self.dmc.enabled = value & 0x10 != 0;

        if !self.pulse1.enabled {
            self.pulse1.length = 0;
        }

        if !self.pulse2.enabled {
            self.pulse2.length = 0;
        }

        if !self.triangle.enabled {
            self.triangle.length = 0;
        }

        if !self.noise.enabled {
            self.noise.length = 0;
        }

        if !self.dmc.enabled {
            self.dmc.remaining = 0;
        }
    }

    fn status(&self) -> u8 {
        let mut result = 0;

        if self.pulse1.length > 0 {
            result |= 0x01;
        }

        if self.pulse2.length > 0 {
            result |= 0x02;
        }

        if self.triangle.length > 0 {
            result |= 0x04;
        }

        if self.noise.length > 0 {
            result |= 0x08;
        }

        if self.dmc.remaining > 0 {
            result |= 0x10;
        }

        result
    }

    pub fn take_samples(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.samples)
    }
}
