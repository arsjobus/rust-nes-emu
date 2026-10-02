use sdl2::audio::{AudioQueue, AudioSpecDesired};

/// Hard cap on queued audio (~200 ms at 48 kHz). The previous cap of a full
/// second meant a runaway queue could add up to a second of latency.
const MAX_QUEUED_SAMPLES: u32 = 9_600;
/// Queue depth the rate controller aims for (~50 ms).
const TARGET_QUEUED_SAMPLES: f64 = 2_400.0;

pub struct Audio {
    queue: AudioQueue<f32>,
    sample_rate: u32,
}

impl Audio {
    pub fn new(audio: &sdl2::AudioSubsystem) -> Result<Self, String> {
        let desired = AudioSpecDesired {
            freq: Some(48_000),
            channels: Some(1),
            samples: Some(1024),
        };
        let queue = audio.open_queue::<f32, _>(None, &desired)?;
        let sample_rate = queue.spec().freq as u32;
        println!(
            "Audio sample rate: {sample_rate} Hz, channels: {}",
            queue.spec().channels
        );
        queue.resume();
        Ok(Self { queue, sample_rate })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn queued_samples(&self) -> u32 {
        self.queue.size() / std::mem::size_of::<f32>() as u32
    }

    /// Factor to trim the APU's output sample rate by so the queue level
    /// hovers around the target. The emulated NES (~60.0988 Hz) and the
    /// host pacing (60 Hz / vsync) never agree exactly; without feedback
    /// the queue drains (crackle) or fills (latency, then dropped samples).
    pub fn rate_adjust(&self) -> f64 {
        let error = (TARGET_QUEUED_SAMPLES - self.queued_samples() as f64) / TARGET_QUEUED_SAMPLES;
        1.0 + (error * 0.005).clamp(-0.005, 0.005)
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        let queued = self.queued_samples();
        let available = MAX_QUEUED_SAMPLES.saturating_sub(queued) as usize;
        if available > 0 {
            if let Err(error) = self
                .queue
                .queue_audio(&samples[..samples.len().min(available)])
            {
                eprintln!("Could not queue audio: {error}");
            }
        }
    }
}
