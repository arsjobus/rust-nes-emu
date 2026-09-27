use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{
DeviceTrait,
HostTrait,
StreamTrait,
};
use cpal::{
SampleFormat,
Stream,
StreamConfig,
};

const MAX_BUFFER_SAMPLES: usize = 48_000;

pub struct Audio {
queue: Arc<Mutex<VecDeque<f32>>>,
channels: usize,
_stream: Stream,
}

impl Audio {
pub fn new() -> Result<Self, String> {
let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or_else(|| {
            "No audio output device found".to_string()
        })?;

    let supported_config = device
        .default_output_config()
        .map_err(|e| {
            format!(
                "Failed to get audio output config: {e}"
            )
        })?;

    let sample_format =
        supported_config.sample_format();

    let config: StreamConfig =
        supported_config.into();

    let channels =
        config.channels as usize;

    println!(
        "Audio device: {}",
        device
            .name()
            .unwrap_or_else(|_| "Unknown".to_string())
    );

    println!(
        "Audio sample rate: {} Hz, channels: {}",
        config.sample_rate.0,
        channels
    );

    println!(
        "Audio sample format: {:?}",
        sample_format
    );

    let queue =
        Arc::new(Mutex::new(
            VecDeque::<f32>::with_capacity(
                MAX_BUFFER_SAMPLES
            )
        ));

    let callback_queue =
        Arc::clone(&queue);

    let error_callback =
        |error| {
            eprintln!(
                "Audio stream error: {error}"
            );
        };

    let stream =
        match sample_format {
            SampleFormat::F32 => {
                device
                    .build_output_stream(
                        &config,
                        move |
                            output: &mut [f32],
                            _,
                        | {
                            write_f32(
                                output,
                                &callback_queue,
                                channels,
                            );
                        },
                        error_callback,
                        None,
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to create \
                             F32 audio stream: {e}"
                        )
                    })?
            }

            SampleFormat::I16 => {
                let callback_queue =
                    Arc::clone(&queue);

                device
                    .build_output_stream(
                        &config,
                        move |
                            output: &mut [i16],
                            _,
                        | {
                            write_i16(
                                output,
                                &callback_queue,
                                channels,
                            );
                        },
                        error_callback,
                        None,
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to create \
                             I16 audio stream: {e}"
                        )
                    })?
            }

            SampleFormat::U16 => {
                let callback_queue =
                    Arc::clone(&queue);

                device
                    .build_output_stream(
                        &config,
                        move |
                            output: &mut [u16],
                            _,
                        | {
                            write_u16(
                                output,
                                &callback_queue,
                                channels,
                            );
                        },
                        error_callback,
                        None,
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to create \
                             U16 audio stream: {e}"
                        )
                    })?
            }

            other => {
                return Err(format!(
                    "Unsupported audio \
                     sample format: {other:?}"
                ));
            }
        };

    stream
        .play()
        .map_err(|e| {
            format!(
                "Failed to start audio stream: {e}"
            )
        })?;

    println!(
        "Audio output initialized"
    );

    Ok(Self {
        queue,
        channels,
        _stream: stream,
    })
}

pub fn push_samples(
    &mut self,
    samples: &[f32],
) {
    let mut queue =
        match self.queue.lock() {
            Ok(queue) => queue,
            Err(_) => return,
        };

    for &sample in samples {
        queue.push_back(
            sample.clamp(-1.0, 1.0)
        );
    }

    while queue.len() > MAX_BUFFER_SAMPLES {
        queue.pop_front();
    }
}


}

fn write_f32(
output: &mut [f32],
queue: &Arc<Mutex<VecDeque<f32>>>,
channels: usize,
) {
let mut queue =
match queue.lock() {
Ok(queue) => queue,
Err(_) => {
output.fill(0.0);
return;
}
};

for frame in output.chunks_mut(channels) {
    let sample =
        queue.pop_front().unwrap_or(0.0);

    for channel in frame.iter_mut() {
        *channel = sample;
    }
}


}

fn write_i16(
output: &mut [i16],
queue: &Arc<Mutex<VecDeque<f32>>>,
channels: usize,
) {
let mut queue =
match queue.lock() {
Ok(queue) => queue,
Err(_) => {
output.fill(0);
return;
}
};

for frame in output.chunks_mut(channels) {
    let value =
        queue.pop_front().unwrap_or(0.0);

    let sample =
        (value.clamp(-1.0, 1.0)
            * i16::MAX as f32)
            as i16;

    for channel in frame.iter_mut() {
        *channel = sample;
    }
}


}

fn write_u16(
output: &mut [u16],
queue: &Arc<Mutex<VecDeque<f32>>>,
channels: usize,
) {
let mut queue =
match queue.lock() {
Ok(queue) => queue,
Err(_) => {
output.fill(u16::MAX / 2);
return;
}
};

for frame in output.chunks_mut(channels) {
    let value =
        queue.pop_front().unwrap_or(0.0);

    let normalized =
        ((value.clamp(-1.0, 1.0) + 1.0) * 0.5)
            .clamp(0.0, 1.0);

    let sample =
        (normalized * u16::MAX as f32) as u16;

    for channel in frame.iter_mut() {
        *channel = sample;
    }
}


}