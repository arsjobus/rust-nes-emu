use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::video::{HEIGHT, WIDTH};

pub struct Recorder {
    video_path: PathBuf,
    audio_path: PathBuf,
    output_path: PathBuf,
    video: BufWriter<File>,
    audio: BufWriter<File>,
    sample_rate: u32,
}

impl Recorder {
    pub fn start(sample_rate: u32) -> Result<Self, String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs();
        let base = format!("runes-recording-{stamp}");
        let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
        let output_path = cwd.join(format!("{base}.mp4"));
        let video_path = cwd.join(format!(".{base}.rgb"));
        let audio_path = cwd.join(format!(".{base}.f32"));
        let video = BufWriter::new(File::create(&video_path).map_err(|error| error.to_string())?);
        let audio = match File::create(&audio_path) {
            Ok(file) => BufWriter::new(file),
            Err(error) => {
                let _ = fs::remove_file(&video_path);
                return Err(error.to_string());
            }
        };
        Ok(Self {
            video_path,
            audio_path,
            output_path,
            video,
            audio,
            sample_rate,
        })
    }

    pub fn write_frame(&mut self, pixels: &[u32]) -> Result<(), String> {
        for pixel in pixels.iter().take(WIDTH * HEIGHT) {
            self.video
                .write_all(&[(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8])
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn write_audio(&mut self, samples: &[f32]) -> Result<(), String> {
        for sample in samples {
            self.audio
                .write_all(&sample.to_le_bytes())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<PathBuf, String> {
        self.video.flush().map_err(|error| error.to_string())?;
        self.audio.flush().map_err(|error| error.to_string())?;
        drop(self.video);
        drop(self.audio);
        let result = Command::new("ffmpeg")
            .args(["-y", "-f", "rawvideo", "-pixel_format", "rgb24"])
            .args([
                "-video_size",
                &format!("{WIDTH}x{HEIGHT}"),
                "-framerate",
                "60",
            ])
            .args(["-i"])
            .arg(&self.video_path)
            .args(["-f", "f32le", "-ar"])
            .arg(self.sample_rate.to_string())
            .args(["-ac", "1", "-i"])
            .arg(&self.audio_path)
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-movflags", "+faststart"])
            .arg(&self.output_path)
            .output()
            .map_err(|error| format!("Could not run ffmpeg: {error}"))?;
        let _ = fs::remove_file(&self.video_path);
        let _ = fs::remove_file(&self.audio_path);
        if !result.status.success() {
            let _ = fs::remove_file(&self.output_path);
            return Err(format!(
                "ffmpeg failed: {}",
                String::from_utf8_lossy(&result.stderr).trim()
            ));
        }
        Ok(self.output_path)
    }
}
