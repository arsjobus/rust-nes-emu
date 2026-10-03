// ============================================================
// DMC - Delta Modulation Channel
// ============================================================
//
// Plays back 1-bit delta-encoded (DPCM) samples read directly
// from CPU address space ($C000-$FFFF). This is what many games
// use for digitized voice clips, drum hits, and sampled ambience
// (Punch-Out!!'s crowd-noise loop and referee/announcer voice
// clips are DMC samples) - so a channel that does nothing, as
// this one previously did, means every game relying on DPCM
// samples is silent (or, if some other bug elsewhere is at play,
// can sound wrong) for that specific content while everything
// else in the game keeps working normally.
//
// Reference: https://www.nesdev.org/wiki/APU_DMC
//
// Known limitation: on real hardware, finishing a non-looping
// sample with the IRQ flag enabled fires a CPU IRQ, and this
// emulator's CPU only implements NMI, not the IRQ line. The flag
// itself is still tracked correctly and exposed via $4015 for any
// game that polls it, but a game that strictly depends on the
// hardware IRQ vector for DMC completion would still be affected.
// This doesn't come up for simple one-shot or looping playback
// (e.g. an ambient crowd loop, which never needs an IRQ at all).

const NTSC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub(super) struct Dmc {
    pub(super) enabled: bool,

    irq_enabled: bool,
    loop_flag: bool,

    // Timer. `period` is in APU cycles (the NTSC rate table is in
    // CPU cycles, and the DMC timer - like pulse/noise - is only
    // clocked on every other CPU cycle).
    timer: u16,
    period: u16,

    // Memory reader.
    sample_address: u16,
    sample_length: u16,
    current_address: u16,
    pub(super) remaining: u16,

    sample_buffer: Option<u8>,

    // Output unit.
    shift_register: u8,
    bits_remaining: u8,
    silence: bool,
    pub(super) output: u8,

    // The APU has no bus access of its own, so a pending DMA read
    // is surfaced here for the bus to service (see
    // `Apu::take_pending_dmc_fetch` / `Apu::feed_dmc_byte`, wired
    // up in `Bus::clock_apu`).
    pending_fetch: Option<u16>,

    pub(super) irq_flag: bool,
}

impl Dmc {
    pub(super) fn new() -> Self {
        Self {
            enabled: false,

            irq_enabled: false,
            loop_flag: false,

            timer: 0,
            period: NTSC_RATE_TABLE[0] / 2,

            sample_address: 0xc000,
            sample_length: 1,
            current_address: 0xc000,
            remaining: 0,

            sample_buffer: None,

            shift_register: 0,
            bits_remaining: 8,
            silence: true,
            output: 0,

            pending_fetch: None,

            irq_flag: false,
        }
    }

    pub(super) fn write(&mut self, reg: u16, value: u8) {
        match reg {
            0 => {
                self.irq_enabled = value & 0x80 != 0;
                self.loop_flag = value & 0x40 != 0;

                if !self.irq_enabled {
                    self.irq_flag = false;
                }

                self.period = NTSC_RATE_TABLE[(value & 0x0f) as usize] / 2;
            }

            1 => {
                // Direct load: immediately sets the output level.
                self.output = value & 0x7f;
            }

            2 => {
                self.sample_address = 0xc000 + (value as u16) * 64;
            }

            3 => {
                self.sample_length = (value as u16) * 16 + 1;
            }

            _ => {}
        }
    }

    // Called from `Apu::set_enable` ($4015 write). Writing $4015
    // always clears the DMC IRQ flag, regardless of the value
    // written. If bit 4 is set and the sample reader is currently
    // idle (bytes_remaining == 0), playback (re)starts from the
    // configured sample address/length; if clear, playback stops
    // immediately (silencing once any buffered bits are used up).
    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.irq_flag = false;

        if !enabled {
            self.remaining = 0;
            self.pending_fetch = None;
        } else if self.remaining == 0 {
            self.restart();
        }

        self.request_fetch_if_needed();
    }

    // Diagnostic snapshot, used by the NES_DEBUG_DMC trace.
    pub(super) fn debug_state(&self) -> String {
        format!(
            "enabled={} loop={} irq_en={} irq={} period={} addr={:04x} len={} cur={:04x} remaining={} silence={} out={}",
            self.enabled,
            self.loop_flag,
            self.irq_enabled,
            self.irq_flag,
            self.period * 2,
            self.sample_address,
            self.sample_length,
            self.current_address,
            self.remaining,
            self.silence,
            self.output,
        )
    }

    fn restart(&mut self) {
        self.current_address = self.sample_address;
        self.remaining = self.sample_length;
    }

    /// The memory reader fetches a byte as soon as the sample buffer is
    /// empty and bytes remain (when playback starts, and the moment the
    /// output unit takes the buffer), not on the next timer tick.
    fn request_fetch_if_needed(&mut self) {
        if self.sample_buffer.is_none() && self.remaining > 0 && self.pending_fetch.is_none() {
            self.pending_fetch = Some(self.current_address);
        }
    }

    // Called once per APU cycle (i.e. every other CPU cycle),
    // matching how pulse/noise timers are clocked in `Apu`.
    pub(super) fn clock_timer(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        // `timer` counts down to 0 and fires on the *next* clock, so a
        // reload of `period - 1` yields exactly `period` APU clocks
        // (= the table's CPU-cycle rate) between output updates.
        self.timer = self.period.saturating_sub(1);

        self.clock_shifter();

        // `Bus::clock_apu` services the request right away.
        self.request_fetch_if_needed();
    }

    fn clock_shifter(&mut self) {
        if !self.silence {
            if self.shift_register & 1 != 0 {
                if self.output <= 125 {
                    self.output += 2;
                }
            } else if self.output >= 2 {
                self.output -= 2;
            }
        }

        self.shift_register >>= 1;

        if self.bits_remaining > 0 {
            self.bits_remaining -= 1;
        }

        if self.bits_remaining == 0 {
            self.bits_remaining = 8;

            match self.sample_buffer.take() {
                Some(byte) => {
                    self.shift_register = byte;
                    self.silence = false;
                }

                None => {
                    // Sample buffer starved (fetch hasn't arrived
                    // yet, or there's nothing left and it isn't
                    // looping): the output level holds steady.
                    self.silence = true;
                }
            }
        }
    }

    // Called by `Bus::clock_apu` with the byte at the address
    // `take_pending_fetch` returned, once it's been read off the
    // CPU bus.
    pub(super) fn take_pending_fetch(&mut self) -> Option<u16> {
        self.pending_fetch.take()
    }

    pub(super) fn feed_byte(&mut self, byte: u8) {
        // Guard against a race: `clock_timer` can request a fetch
        // while `remaining > 0`, but the channel can be disabled
        // (which force-sets `remaining = 0`) before that fetch is
        // actually serviced and this function is called with the
        // resulting byte. Without this guard, the unconditional
        // `remaining -= 1` below would underflow a `u16` from 0,
        // wrapping to 65535 - making the reader think there are
        // another 65535 bytes of "sample" left and causing it to
        // stream raw PRG-ROM bytes as audio indefinitely, ignoring
        // that it was ever disabled. Simplest correct fix: a byte
        // that arrives after we've been reset to idle is stale -
        // just discard it.
        if self.remaining == 0 {
            self.sample_buffer = None;
            return;
        }

        self.sample_buffer = Some(byte);

        // $FFFF wraps to $8000, per hardware (nesdev wiki: "the
        // address is incremented; if it exceeds $FFFF, it wraps
        // to $8000").
        self.current_address = if self.current_address == 0xffff {
            0x8000
        } else {
            self.current_address + 1
        };

        self.remaining -= 1;

        if self.remaining == 0 {
            if self.loop_flag {
                self.restart();
            } else if self.irq_enabled {
                self.irq_flag = true;
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/apu/dmc.rs"]
mod tests;
