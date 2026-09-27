use std::f64;

const CPU_CLOCK: f64 = 1_789_773.0;
const SAMPLE_RATE: f64 = 48_000.0;

const PULSE_DUTY: [[f32; 8]; 4] = [
[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
[0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
[0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
[1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0],
];

const LENGTH_TABLE: [u8; 32] = [
10, 254, 20, 2,
40, 4, 80, 6,
160, 8, 60, 10,
14, 12, 26, 14,
12, 16, 24, 18,
48, 20, 96, 22,
192, 24, 72, 26,
16, 28, 32, 30,
];

const NOISE_PERIODS: [u16; 16] = [
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

        0x4015 => {
            self.set_enable(value);
        }

        0x4017 => {
            self.frame_counter.write(value);
        }

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

    /*
     * Frame counter.
     *
     * The APU frame counter runs independently of the
     * individual channel timers.
     */
    self.frame_counter.clock(
        &mut self.pulse1,
        &mut self.pulse2,
        &mut self.triangle,
        &mut self.noise,
    );

    /*
     * Pulse and noise timers run at half CPU frequency.
     */
    if self.cpu_cycle & 1 == 0 {
        self.pulse1.clock_timer();
        self.pulse2.clock_timer();
        self.noise.clock_timer();
    }

    /*
     * Triangle timer runs every CPU cycle.
     */
    self.triangle.clock_timer();

    /*
     * Generate 44.1 kHz samples from the NES CPU clock.
     *
     * Using f64 here avoids accumulating noticeable timing
     * error over long play sessions.
     */
    self.sample_clock += SAMPLE_RATE / CPU_CLOCK;

    while self.sample_clock >= 1.0 {
        self.sample_clock -= 1.0;

        self.samples.push(self.mix());
    }
}

fn mix(&self) -> f32 {
    let p1 = self.pulse1.output();
    let p2 = self.pulse2.output();

    /*
     * NES pulse mixer.
     */
    let pulse = p1 + p2;

    let pulse_out = if pulse <= 0.0 {
        0.0
    } else {
        95.88 / ((8128.0 / pulse) + 100.0)
    };

    /*
     * NES triangle/noise/DMC mixer.
     */
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

    /*
     * The APU mixer is naturally unipolar.
     *
     * Keep this normalized rather than artificially
     * centering it, because the audio backend can handle
     * the final sample conversion.
     */
    (pulse_out + tnd_out).clamp(0.0, 1.0) as f32
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

/* ============================================================

    PULSE

    ============================================================
    */

struct Pulse {
enabled: bool,

duty: usize,
sequence: usize,

timer: u16,
period: u16,

length: u8,

volume: u8,
constant_volume: bool,

envelope_start: bool,
envelope_divider: u8,
envelope_decay: u8,
envelope_loop: bool,

sweep_enabled: bool,
sweep_negate: bool,
sweep_shift: u8,
sweep_divider: u8,
sweep_reload: bool,
sweep_period: u8,

channel_one: bool,

}

impl Pulse {
fn new(channel_one: bool) -> Self {
Self {
enabled: false,

        duty: 0,
        sequence: 0,

        timer: 0,
        period: 0,

        length: 0,

        volume: 0,
        constant_volume: false,

        envelope_start: false,
        envelope_divider: 0,
        envelope_decay: 0,
        envelope_loop: false,

        sweep_enabled: false,
        sweep_negate: false,
        sweep_shift: 0,
        sweep_divider: 0,
        sweep_reload: false,
        sweep_period: 0,

        channel_one,
    }
}

fn write(&mut self, reg: u16, value: u8) {
    match reg {
        0 => {
            self.duty = ((value >> 6) & 3) as usize;

            self.envelope_loop =
                value & 0x20 != 0;

            self.constant_volume =
                value & 0x10 != 0;

            self.volume =
                value & 0x0f;
        }

        1 => {
            self.sweep_enabled =
                value & 0x80 != 0;

            self.sweep_period =
                ((value >> 4) & 7) + 1;

            self.sweep_negate =
                value & 0x08 != 0;

            self.sweep_shift =
                value & 7;

            self.sweep_reload = true;
        }

        2 => {
            self.period =
                (self.period & 0x0700)
                | value as u16;
        }

        3 => {
            self.period =
                (self.period & 0x00ff)
                | (((value & 7) as u16) << 8);

            self.length =
                LENGTH_TABLE[(value >> 3) as usize];

            self.sequence = 0;

            self.envelope_start = true;
        }

        _ => {}
    }
}

fn clock_timer(&mut self) {
    if self.timer == 0 {
        self.timer = self.period;

        self.sequence =
            (self.sequence + 1) & 7;
    } else {
        self.timer -= 1;
    }
}

fn clock_length(&mut self) {
    if !self.envelope_loop &&
       self.length > 0
    {
        self.length -= 1;
    }
}

fn clock_envelope(&mut self) {
    if self.envelope_start {
        self.envelope_start = false;
        self.envelope_decay = 15;
        self.envelope_divider = self.volume;
        return;
    }

    if self.envelope_divider > 0 {
        self.envelope_divider -= 1;
    } else {
        self.envelope_divider = self.volume;

        if self.envelope_decay > 0 {
            self.envelope_decay -= 1;
        } else if self.envelope_loop {
            self.envelope_decay = 15;
        }
    }
}

fn clock_sweep(&mut self) {
    if self.sweep_divider == 0 {
        if self.sweep_enabled &&
           self.sweep_shift != 0
        {
            let change =
                self.period >> self.sweep_shift;

            if self.sweep_negate {
                self.period =
                    self.period
                        .wrapping_sub(change)
                        .wrapping_sub(
                            if self.channel_one {
                                1
                            } else {
                                0
                            },
                        );
            } else {
                self.period =
                    self.period.wrapping_add(change);
            }
        }

        self.sweep_divider =
            self.sweep_period;
    } else {
        self.sweep_divider -= 1;
    }

    if self.sweep_reload {
        self.sweep_reload = false;
        self.sweep_divider =
            self.sweep_period;
    }
}

fn output(&self) -> f32 {
    if !self.enabled ||
       self.length == 0 ||
       self.period < 8 ||
       self.period > 0x7ff
    {
        return 0.0;
    }

    let duty =
        PULSE_DUTY[self.duty][self.sequence];

    if duty == 0.0 {
        return 0.0;
    }

    let volume =
        if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        };

    duty * volume as f32
}

}

/* ============================================================

    TRIANGLE

    ============================================================
    */

struct Triangle {
enabled: bool,

timer: u16,
period: u16,

length: u8,

sequence: usize,

linear_counter: u8,
linear_reload_value: u8,
linear_reload: bool,
control_flag: bool,

}

impl Triangle {
fn new() -> Self {
Self {
enabled: false,

        timer: 0,
        period: 0,

        length: 0,

        sequence: 0,

        linear_counter: 0,
        linear_reload_value: 0,
        linear_reload: false,
        control_flag: false,
    }
}

fn write(&mut self, reg: u16, value: u8) {
    match reg {
        0 => {
            self.control_flag =
                value & 0x80 != 0;

            self.linear_reload_value =
                value & 0x7f;
        }

        2 => {
            self.period =
                (self.period & 0x0700)
                | value as u16;
        }

        3 => {
            self.period =
                (self.period & 0x00ff)
                | (((value & 7) as u16) << 8);

            self.length =
                LENGTH_TABLE[(value >> 3) as usize];

            self.linear_reload = true;
        }

        _ => {}
    }
}

fn clock_timer(&mut self) {
    if self.timer == 0 {
        self.timer = self.period;

        /*
         * The triangle sequencer only advances when
         * both the length counter and linear counter
         * are active.
         */
        if self.length > 0 &&
           self.linear_counter > 0
        {
            self.sequence =
                (self.sequence + 1) & 31;
        }
    } else {
        self.timer -= 1;
    }
}

fn clock_length(&mut self) {
    if !self.control_flag &&
       self.length > 0
    {
        self.length -= 1;
    }
}

fn clock_linear_counter(&mut self) {
    if self.linear_reload {
        self.linear_counter =
            self.linear_reload_value;
    } else if self.linear_counter > 0 {
        self.linear_counter -= 1;
    }

    if !self.control_flag {
        self.linear_reload = false;
    }
}

fn output(&self) -> f32 {
    if !self.enabled ||
       self.length == 0 ||
       self.linear_counter == 0
    {
        return 0.0;
    }

    let x = self.sequence as f32;

    if x < 16.0 {
        x
    } else {
        31.0 - x
    }
}

}

/* ============================================================

    NOISE

    ============================================================
    */

struct Noise {
enabled: bool,

length: u8,

mode: bool,
period_index: usize,
timer: u16,

shift: u16,

volume: u8,
constant_volume: bool,

envelope_start: bool,
envelope_decay: u8,
envelope_divider: u8,
envelope_loop: bool,

}

impl Noise {
fn new() -> Self {
Self {
enabled: false,

        length: 0,

        mode: false,
        period_index: 0,
        timer: 0,

        shift: 1,

        volume: 0,
        constant_volume: false,

        envelope_start: false,
        envelope_decay: 0,
        envelope_divider: 0,
        envelope_loop: false,
    }
}

fn write(&mut self, reg: u16, value: u8) {
    match reg {
        0 => {
            self.envelope_loop =
                value & 0x20 != 0;

            self.constant_volume =
                value & 0x10 != 0;

            self.volume =
                value & 0x0f;
        }

        2 => {
            self.mode =
                value & 0x80 != 0;

            self.period_index =
                (value & 0x0f) as usize;
        }

        3 => {
            self.length =
                LENGTH_TABLE[(value >> 3) as usize];

            self.envelope_start = true;
        }

        _ => {}
    }
}

fn clock_timer(&mut self) {
    if self.timer == 0 {
        self.timer =
            NOISE_PERIODS[self.period_index];

        let tap =
            if self.mode { 6 } else { 1 };

        let feedback =
            (self.shift & 1)
            ^ ((self.shift >> tap) & 1);

        self.shift >>= 1;
        self.shift |= feedback << 14;
    } else {
        self.timer -= 1;
    }
}

fn clock_length(&mut self) {
    if !self.envelope_loop &&
       self.length > 0
    {
        self.length -= 1;
    }
}

fn clock_envelope(&mut self) {
    if self.envelope_start {
        self.envelope_start = false;
        self.envelope_decay = 15;
        self.envelope_divider = self.volume;
        return;
    }

    if self.envelope_divider > 0 {
        self.envelope_divider -= 1;
    } else {
        self.envelope_divider = self.volume;

        if self.envelope_decay > 0 {
            self.envelope_decay -= 1;
        } else if self.envelope_loop {
            self.envelope_decay = 15;
        }
    }
}

fn output(&self) -> f32 {
    if !self.enabled ||
       self.length == 0 ||
       self.shift & 1 != 0
    {
        return 0.0;
    }

    if self.constant_volume {
        self.volume as f32
    } else {
        self.envelope_decay as f32
    }
}

}

/* ============================================================

    DMC

    ============================================================

    Still intentionally stubbed.

    This keeps the current sound path intact without introducing

    a second large source of bugs.
    */

struct Dmc {
enabled: bool,
remaining: u16,
output: u8,
}

impl Dmc {
fn new() -> Self {
Self {
enabled: false,
remaining: 0,
output: 0,
}
}

fn write(&mut self, _reg: u16, _value: u8) {}

}

/* ============================================================

    FRAME COUNTER

    ============================================================
    */

struct FrameCounter {
cycle: u32,
mode_5: bool,
}

impl FrameCounter {
fn new() -> Self {
Self {
cycle: 0,
mode_5: false,
}
}

fn write(&mut self, value: u8) {
    self.mode_5 = value & 0x80 != 0;

    /*
     * Writing $4017 resets the frame counter.
     */
    self.cycle = 0;
}

fn clock(
    &mut self,
    p1: &mut Pulse,
    p2: &mut Pulse,
    triangle: &mut Triangle,
    noise: &mut Noise,
) {
    self.cycle += 1;

    /*
     * NTSC APU frame-counter timing is approximately
     * 3729 CPU cycles per quarter-frame.
     *
     * Using 3729 keeps this compatible with the timing
     * model already used by the emulator.
     */
    const QUARTER: u32 = 3729;

    /*
     * Quarter-frame clocks:
     *
     * - pulse envelopes
     * - noise envelope
     * - triangle linear counter
     */
    if self.cycle % QUARTER == 0 {
        p1.clock_envelope();
        p2.clock_envelope();
        noise.clock_envelope();

        triangle.clock_linear_counter();
    }

    /*
     * Half-frame clocks:
     *
     * - length counters
     * - pulse sweep
     */
    if self.cycle % (QUARTER * 2) == 0 {
        p1.clock_length();
        p2.clock_length();

        triangle.clock_length();

        noise.clock_length();

        p1.clock_sweep();
        p2.clock_sweep();
    }

    /*
     * 4-step sequence.
     */
    if !self.mode_5 &&
       self.cycle >= QUARTER * 4
    {
        self.cycle = 0;
    }

    /*
     * 5-step sequence.
     *
     * The final step does not clock the envelope/length
     * units in this simplified timing model.
     */
    if self.mode_5 &&
       self.cycle >= QUARTER * 5
    {
        self.cycle = 0;
    }
}

}