use sdl2::audio::{AudioQueue, AudioSpecDesired};

const MAX_QUEUED_SAMPLES: u32 = 48_000;

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

    pub fn push_samples(&mut self, samples: &[f32]) {
        let queued = self.queue.size() / std::mem::size_of::<f32>() as u32;
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
