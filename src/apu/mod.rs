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
const DEFAULT_SAMPLE_RATE: f64 = 48_000.0;

pub const PULSE_DUTY: [[f32; 8]; 4] = [
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0],
];

pub const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

pub const NOISE_PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub struct Apu {
    pulse1: Pulse,
    pulse2: Pulse,
    mmc5_pulse1: Pulse,
    mmc5_pulse2: Pulse,
    mmc5_enabled: u8,
    mmc5_pcm_mode: bool,
    mmc5_pcm_irq_enabled: bool,
    mmc5_pcm_irq: bool,
    mmc5_pcm_value: u8,
    mmc5_pcm_active: bool,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,

    frame_counter: FrameCounter,

    cpu_cycle: u64,

    // Must match the actual audio output device's sample rate
    // exactly (see `Audio::sample_rate` / `set_sample_rate`
    // below) - generating audio at a rate that doesn't match what
    // the device is driven at causes the producer/consumer sides
    // of the playback queue to drift apart over time, eventually
    // causing periodic, audible clicking as the queue overflows
    // or underflows.
    sample_rate: f64,

    output_filter: OutputFilter,

    sample_clock: f64,
    sample_accum: f32,
    sample_accum_count: u32,
    /// Tiny multiplier on the output sample rate, used by the frontend to
    /// keep the audio queue level steady (see `set_rate_adjust`).
    rate_adjust: f64,
    samples: Vec<f32>,

    // Per-channel mute switches, controlled by environment
    // variables (NES_MUTE_PULSE1=1, NES_MUTE_PULSE2=1,
    // NES_MUTE_TRIANGLE=1, NES_MUTE_NOISE=1, NES_MUTE_DMC=1) - a
    // quick diagnostic for isolating which channel is responsible
    // for an unexpected sound, without needing to add print
    // debugging or step through code. Not meant as a permanent
    // feature, just a fast way to bisect "which channel is that."
    mute_pulse1: bool,
    mute_pulse2: bool,
    mute_triangle: bool,
    mute_noise: bool,
    mute_dmc: bool,

    // NES_DEBUG_DMC=1 prints every DMC-related register write
    // (with the CPU cycle it happened on) plus a once-per-second
    // summary of DMC activity to stderr.
    debug_dmc: bool,
    debug_fetches: u32,
    debug_lines: u32,

    // NES_TRACE_FILE=path writes every APU register write and any
    // events reported via `debug_event` (e.g. NMI) with the CPU
    // cycle, and NES_DUMP_WAV=path saves the first 20 s of final
    // output audio as a 16-bit mono WAV. Both are for diagnosis.
    trace_file: Option<std::io::BufWriter<std::fs::File>>,
    wav_path: Option<String>,
    wav_samples: Vec<i16>,
}

// First-order filters modelling the NES's analog output stage: two
// high-pass filters (~90 Hz and ~442 Hz) and a ~14 kHz low-pass.
// The high-pass stages are what remove DC on real hardware - without
// them, a channel that parks at a non-zero level (e.g. the DMC after
// software PCM playback via $4011, which Punch-Out uses for the crowd)
// leaves a constant offset in the mix, and any gap in audio delivery
// then becomes an audible click.
#[derive(Clone, Copy)]
struct HighPass {
    alpha: f32,
    prev_in: f32,
    prev_out: f32,
}

impl HighPass {
    fn new(cutoff: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;

        Self {
            alpha: rc / (rc + dt),
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }

    fn apply(&mut self, input: f32) -> f32 {
        let out = self.alpha * (self.prev_out + input - self.prev_in);
        self.prev_in = input;
        self.prev_out = out;
        out
    }
}

#[derive(Clone, Copy)]
struct LowPass {
    alpha: f32,
    prev_out: f32,
}

impl LowPass {
    fn new(cutoff: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;

        Self {
            alpha: dt / (rc + dt),
            prev_out: 0.0,
        }
    }

    fn apply(&mut self, input: f32) -> f32 {
        self.prev_out += self.alpha * (input - self.prev_out);
        self.prev_out
    }
}

#[derive(Clone, Copy)]
struct OutputFilter {
    hp90: HighPass,
    hp442: HighPass,
    lp14k: LowPass,
}

impl OutputFilter {
    fn new(sample_rate: f64) -> Self {
        let sr = sample_rate as f32;

        Self {
            hp90: HighPass::new(90.0, sr),
            hp442: HighPass::new(442.0, sr),
            lp14k: LowPass::new(14_000.0_f32.min(sr * 0.45), sr),
        }
    }

    fn apply(&mut self, input: f32) -> f32 {
        let x = self.hp90.apply(input);
        let x = self.hp442.apply(x);
        self.lp14k.apply(x)
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).map(|v| v == "1").unwrap_or(false)
}

impl Apu {
    pub fn new() -> Self {
        Self {
            pulse1: Pulse::new(true),
            pulse2: Pulse::new(false),
            mmc5_pulse1: Pulse::new_mmc5(),
            mmc5_pulse2: Pulse::new_mmc5(),
            mmc5_enabled: 0,
            mmc5_pcm_mode: false,
            mmc5_pcm_irq_enabled: false,
            mmc5_pcm_irq: false,
            mmc5_pcm_value: 0,
            mmc5_pcm_active: false,
            triangle: Triangle::new(),
            noise: Noise::new(),
            dmc: Dmc::new(),

            frame_counter: FrameCounter::power_on(),

            cpu_cycle: 0,
            sample_rate: DEFAULT_SAMPLE_RATE,
            output_filter: OutputFilter::new(DEFAULT_SAMPLE_RATE),
            sample_clock: 0.0,
            sample_accum: 0.0,
            sample_accum_count: 0,
            rate_adjust: 1.0,
            samples: Vec::with_capacity(2048),

            mute_pulse1: env_flag("NES_MUTE_PULSE1"),
            mute_pulse2: env_flag("NES_MUTE_PULSE2"),
            mute_triangle: env_flag("NES_MUTE_TRIANGLE"),
            mute_noise: env_flag("NES_MUTE_NOISE"),
            mute_dmc: env_flag("NES_MUTE_DMC"),
            debug_dmc: env_flag("NES_DEBUG_DMC"),
            debug_fetches: 0,
            debug_lines: 0,
            trace_file: std::env::var("NES_TRACE_FILE")
                .ok()
                .and_then(|p| std::fs::File::create(p).ok().map(std::io::BufWriter::new)),
            wav_path: std::env::var("NES_DUMP_WAV").ok(),
            wav_samples: Vec::new(),
        }
    }

    // Called once, as soon as the real audio output device is
    // opened and its actual sample rate is known (before the main
    // loop starts running frames), so that generated audio always
    // matches what's actually being played back.
    /// Trim the produced sample rate by a small factor (clamped to +/-1%).
    /// The NES runs at ~60.0988 Hz while the host pacing is usually 60 Hz,
    /// so without feedback the audio queue slowly drains or overfills.
    pub fn set_rate_adjust(&mut self, factor: f64) {
        self.rate_adjust = factor.clamp(0.99, 1.01);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.output_filter = OutputFilter::new(sample_rate);
    }

    /// Warm reset: all channels are silenced as if $4015 were written
    /// with 0, the frame IRQ flag is cleared and the frame counter
    /// restarts as if the last value written to $4017 were written again.
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.set_enable(0);
        self.frame_counter.reset();
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let status = self.status();
                // Reading $4015 acknowledges the frame IRQ (but not
                // the DMC IRQ, which is cleared through $4015 writes).
                self.frame_counter.irq_flag = false;
                status
            }
            0x5010 => {
                let v = ((self.mmc5_pcm_irq && self.mmc5_pcm_irq_enabled) as u8) << 7 | 1;
                self.mmc5_pcm_irq = false;
                v
            }
            0x5015 => {
                (self.mmc5_pulse1.length > 0) as u8 | (((self.mmc5_pulse2.length > 0) as u8) << 1)
            }
            _ => 0,
        }
    }

    /// CPU cycles clocked so far (its parity is the APU's phase).
    pub fn cycle_count(&self) -> u64 {
        self.cpu_cycle
    }

    pub fn debug_event(&mut self, label: &str) {
        if let Some(file) = self.trace_file.as_mut() {
            use std::io::Write;
            let _ = writeln!(file, "{} {}", self.cpu_cycle, label);
        }
    }

    fn write_wav(&mut self) {
        use std::io::Write;

        let Some(path) = self.wav_path.take() else {
            return;
        };
        let rate = self.sample_rate as u32;
        let data_len = (self.wav_samples.len() * 2) as u32;

        let mut bytes = Vec::with_capacity(44 + data_len as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for s in &self.wav_samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }

        if let Ok(mut f) = std::fs::File::create(&path) {
            let _ = f.write_all(&bytes);
            eprintln!("[apu] wrote {}", path);
        }
        self.wav_samples = Vec::new();
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        if let Some(file) = self.trace_file.as_mut() {
            use std::io::Write;
            if matches!(addr, 0x4000..=0x4017 | 0x5000..=0x5015) {
                let _ = writeln!(file, "{} W {:04x} {:02x}", self.cpu_cycle, addr, value);
            }
        }

        if self.debug_dmc && matches!(addr, 0x4010..=0x4013 | 0x4015) && self.debug_lines < 300 {
            self.debug_lines += 1;
            eprintln!(
                "[dmc] cycle {:>10}: write ${:04x} = {:02x}",
                self.cpu_cycle, addr, value
            );
        }

        match addr {
            0x5000..=0x5003 => self.mmc5_pulse1.write(addr - 0x5000, value),
            0x5004..=0x5007 => self.mmc5_pulse2.write(addr - 0x5004, value),
            0x5010 => {
                self.mmc5_pcm_mode = value & 1 != 0;
                self.mmc5_pcm_irq_enabled = value & 0x80 != 0;
            }
            0x5011 => {
                if !self.mmc5_pcm_mode {
                    self.mmc5_pcm_value = value;
                    self.mmc5_pcm_active = true;
                    self.mmc5_pcm_irq = value == 0;
                }
            }
            0x5015 => {
                self.mmc5_enabled = value & 3;
                self.mmc5_pulse1.enabled = value & 1 != 0;
                self.mmc5_pulse2.enabled = value & 2 != 0;
                if value & 1 == 0 {
                    self.mmc5_pulse1.length = 0;
                }
                if value & 2 == 0 {
                    self.mmc5_pulse2.length = 0;
                }
            }
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

                if self.debug_dmc && self.debug_lines < 300 {
                    eprintln!("[dmc]   -> {}", self.dmc.debug_state());
                }
            }
            0x4017 => self.frame_counter.write_at(value, self.cpu_cycle),
            _ => {}
        }
    }

    // Called once per CPU cycle by `Bus::clock_apu`, which also
    // services any pending DMC DMA read between cycles (the APU
    // itself has no bus access to do that directly).
    pub(crate) fn step_cycle(&mut self) {
        self.cpu_cycle += 1;

        self.frame_counter.clock(
            &mut self.pulse1,
            &mut self.pulse2,
            &mut self.mmc5_pulse1,
            &mut self.mmc5_pulse2,
            &mut self.triangle,
            &mut self.noise,
        );

        if self.cpu_cycle & 1 == 0 {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            self.dmc.clock_timer();
            self.mmc5_pulse1.clock_timer();
            self.mmc5_pulse2.clock_timer();
        }

        // The triangle and noise timers are clocked every CPU
        // cycle. (NOISE_PERIODS is already expressed in CPU
        // cycles, so clocking noise on alternate cycles like the
        // pulse channels would double every period and drop all
        // noise an octave.)
        self.triangle.clock_timer();
        self.noise.clock_timer();

        let second_boundary =
            (self.trace_file.is_some() || self.debug_dmc) && self.cpu_cycle % 1_789_773 == 0;

        if second_boundary {
            if let Some(file) = self.trace_file.as_mut() {
                use std::io::Write;
                let _ = file.flush();
            }
        }

        if self.debug_dmc && second_boundary {
            eprintln!(
                "[dmc] t={}s fetches/s={} {}",
                self.cpu_cycle / 1_789_773,
                self.debug_fetches,
                self.dmc.debug_state()
            );
            self.debug_fetches = 0;
        }

        // Average every CPU-cycle mix value that falls inside one output
        // sample (a box filter). Point-sampling the 1.79 MHz signal at
        // 48 kHz aliases the fast pulse/noise/DMC edges into audible hiss.
        self.sample_accum += self.mix();
        self.sample_accum_count += 1;
        self.sample_clock += self.sample_rate * self.rate_adjust / CPU_CLOCK;

        while self.sample_clock >= 1.0 {
            self.sample_clock -= 1.0;
            let mixed = self.sample_accum / self.sample_accum_count.max(1) as f32;
            self.sample_accum = 0.0;
            self.sample_accum_count = 0;
            let filtered = self.output_filter.apply(mixed);
            let out = filtered.clamp(-1.0, 1.0);
            self.samples.push(out);

            if self.wav_path.is_some() {
                self.wav_samples.push((out * 32767.0) as i16);

                if self.wav_samples.len() >= (self.sample_rate as usize) * 20 {
                    self.write_wav();
                }
            }
        }
    }

    fn mix(&self) -> f32 {
        let p1 = if self.mute_pulse1 {
            0.0
        } else {
            self.pulse1.output()
        };
        let p2 = if self.mute_pulse2 {
            0.0
        } else {
            self.pulse2.output()
        };

        let pulse = p1 + p2 + self.mmc5_pulse1.mmc5_output() + self.mmc5_pulse2.mmc5_output();

        let pulse_out = if pulse <= 0.0 {
            0.0
        } else {
            95.88 / ((8128.0 / pulse) + 100.0)
        };

        let triangle = if self.mute_triangle {
            0.0
        } else {
            self.triangle.output()
        };
        let noise = if self.mute_noise {
            0.0
        } else {
            self.noise.output()
        };
        let dmc = if self.mute_dmc {
            0.0
        } else {
            self.dmc.output as f32
        };

        let tnd = triangle / 8227.0 + noise / 12241.0 + dmc / 22638.0;

        let tnd_out = if tnd <= 0.0 {
            0.0
        } else {
            159.79 / ((1.0 / tnd) + 100.0)
        };

        let pcm = if self.mmc5_pcm_active {
            (255 - self.mmc5_pcm_value) as f32 / 510.0
        } else {
            0.0
        };
        (pulse_out + tnd_out + pcm).clamp(0.0, 1.0)
    }

    fn set_enable(&mut self, value: u8) {
        self.pulse1.enabled = value & 0x01 != 0;
        self.pulse2.enabled = value & 0x02 != 0;
        self.triangle.enabled = value & 0x04 != 0;
        self.noise.enabled = value & 0x08 != 0;
        self.dmc.set_enabled(value & 0x10 != 0);

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

        if self.frame_counter.irq_flag {
            result |= 0x40;
        }

        if self.dmc.irq_flag {
            result |= 0x80;
        }

        result
    }

    /*
     * ---------------------------------------------------------
     * DMC DMA bridge
     * ---------------------------------------------------------
     *
     * The APU has no bus access of its own (it's a field of
     * `Bus`, so it can't hold a reference back to it), but the
     * DMC channel needs to read sample bytes directly from CPU
     * address space. `Bus::clock_apu` polls this after every
     * cycle and, when a fetch is pending, performs the actual
     * `Bus::read` and feeds the byte back in.
     */
    // True while any modeled APU interrupt source is asserting IRQ.
    pub fn irq_line(&self) -> bool {
        self.frame_counter.irq_flag
            || self.dmc.irq_flag
            || self.mmc5_pcm_irq && self.mmc5_pcm_irq_enabled
    }

    pub fn take_pending_dmc_fetch(&mut self) -> Option<u16> {
        self.dmc.take_pending_fetch()
    }

    pub fn feed_dmc_byte(&mut self, byte: u8) {
        self.debug_fetches += 1;
        self.dmc.feed_byte(byte);
    }

    pub fn mmc5_pcm_read(&mut self, byte: u8) {
        if self.mmc5_pcm_mode {
            if byte == 0 {
                self.mmc5_pcm_irq = true;
            } else {
                self.mmc5_pcm_value = byte;
                self.mmc5_pcm_active = true;
            }
        }
    }

    pub fn take_samples(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.samples)
    }
}
