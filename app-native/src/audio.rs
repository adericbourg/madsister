//! Playing the song's audio (F-PB-1): ffmpeg decodes the file into memory, at the output device's rate and channel
//! count, and cpal plays it. ffmpeg is already a prerequisite of the engine on every platform, and reads every format
//! the import produces (Opus from a URL included).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

/// The ffmpeg next to the app (the Windows build ships it), else the one on PATH.
fn ffmpeg() -> PathBuf {
    let bundled = std::env::current_exe().ok().and_then(|exe| Some(exe.parent()?.join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX))));
    bundled.filter(|path| path.exists()).unwrap_or_else(|| "ffmpeg".into())
}

/// The file's samples, interleaved, at `rate` Hz and `channels` channels.
pub fn decode(path: &Path, rate: u32, channels: u16) -> Result<Vec<f32>, String> {
    let mut command = Command::new(ffmpeg());
    command
        .args(["-nostdin", "-loglevel", "error", "-i"])
        .arg(path)
        .args(["-f", "f32le", "-acodec", "pcm_f32le", "-ac", &channels.to_string(), "-ar", &rate.to_string(), "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // No console window popping up (the app is a GUI program).
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
    let output = command.output().map_err(|e| format!("can't run ffmpeg: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.lines().last().unwrap_or("ffmpeg failed").to_string());
    }
    Ok(output.stdout.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect())
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Loading,
    Ready,
    Failed(String),
}

struct Shared {
    // shortcut: the whole song decoded in memory (~115 MB for 5 minutes at 48 kHz stereo); decode in chunks while
    // playing if long recordings become common.
    samples: Vec<f32>,
    /// The next frame to play.
    frame: usize,
    is_playing: bool,
    status: Status,
}

pub struct Player {
    shared: Arc<Mutex<Shared>>,
    /// Kept alive while the player is: dropping it closes the output.
    _stream: cpal::Stream,
    rate: u32,
    channels: usize,
}

impl Player {
    /// Opens the default output and decodes `path` in the background; `status` says when it's ready.
    pub fn open(path: &Path) -> Result<Player, String> {
        let device = cpal::default_host().default_output_device().ok_or("no audio output")?;
        let config = device.default_output_config().map_err(|e| format!("no audio output: {e}"))?;
        let (rate, channels) = (config.sample_rate(), config.channels());
        let shared = Arc::new(Mutex::new(Shared { samples: Vec::new(), frame: 0, is_playing: false, status: Status::Loading }));
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => stream::<f32>(&device, config.into(), &shared),
            cpal::SampleFormat::I16 => stream::<i16>(&device, config.into(), &shared),
            cpal::SampleFormat::I32 => stream::<i32>(&device, config.into(), &shared),
            cpal::SampleFormat::U16 => stream::<u16>(&device, config.into(), &shared),
            format => return Err(format!("unsupported audio output format {format}")),
        }?;
        let loading = shared.clone();
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let decoded = decode(&path, rate, channels);
            let mut shared = loading.lock().unwrap();
            match decoded {
                Ok(samples) => {
                    shared.samples = samples;
                    shared.status = Status::Ready;
                }
                Err(e) => shared.status = Status::Failed(format!("Can't play this audio: {e}")),
            }
        });
        Ok(Player { shared, _stream: stream, rate, channels: usize::from(channels) })
    }

    pub fn status(&self) -> Status {
        self.shared.lock().unwrap().status.clone()
    }

    pub fn is_playing(&self) -> bool {
        self.shared.lock().unwrap().is_playing
    }

    /// Seconds from the start.
    pub fn time(&self) -> f64 {
        self.shared.lock().unwrap().frame as f64 / f64::from(self.rate)
    }

    pub fn duration(&self) -> f64 {
        (self.shared.lock().unwrap().samples.len() / self.channels) as f64 / f64::from(self.rate)
    }

    /// Plays or pauses; false when there's nothing to play yet.
    pub fn toggle(&self) -> bool {
        let mut shared = self.shared.lock().unwrap();
        if shared.status != Status::Ready {
            return false;
        }
        let frames = shared.samples.len() / self.channels;
        if !shared.is_playing && shared.frame >= frames {
            shared.frame = 0; // played to the end: from the start again
        }
        shared.is_playing = !shared.is_playing;
        true
    }

    /// Moves the playback position (the bar cursor follows, even while paused); false when there's nothing to play.
    pub fn seek(&self, sec: f64) -> bool {
        let mut shared = self.shared.lock().unwrap();
        if shared.status != Status::Ready {
            return false;
        }
        let frames = shared.samples.len() / self.channels;
        shared.frame = ((sec.max(0.0) * f64::from(self.rate)) as usize).min(frames);
        true
    }
}

/// The output stream, running: it plays the shared samples from the current frame, silence when paused or past the end.
// shortcut: it runs while paused, writing silence: PipeWire's ALSA layer refuses to pause a stream (snd_pcm_pause). Drop and
// rebuild the stream on pause if the idle cost ever matters.
fn stream<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    shared: &Arc<Mutex<Shared>>,
) -> Result<cpal::Stream, String> {
    let channels = usize::from(config.channels);
    let shared = shared.clone();
    let write = move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
        let mut shared = shared.lock().unwrap();
        let start = shared.frame * channels;
        let available = if shared.is_playing { shared.samples.len().saturating_sub(start) } else { 0 };
        let count = available.min(out.len());
        for (o, s) in out.iter_mut().zip(&shared.samples[start..start + count]) {
            *o = T::from_sample(*s);
        }
        for o in &mut out[count..] {
            *o = T::from_sample(0.0);
        }
        shared.frame += count / channels;
        if shared.is_playing && count < out.len() {
            shared.is_playing = false; // the end
        }
    };
    let stream = device
        .build_output_stream(config, write, |e| eprintln!("audio output: {e}"), None)
        .map_err(|e| format!("no audio output: {e}"))?;
    stream.play().map_err(|e| format!("no audio output: {e}"))?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit WAV of `seconds` of silence at 8 kHz.
    fn silent_wav(seconds: f64) -> Vec<u8> {
        let samples = (8000.0 * seconds) as u32;
        let data = samples * 2;
        let mut wav = Vec::new();
        wav.extend(b"RIFF");
        wav.extend((36 + data).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes()); // PCM
        wav.extend(1u16.to_le_bytes()); // mono
        wav.extend(8000u32.to_le_bytes());
        wav.extend(16000u32.to_le_bytes()); // byte rate
        wav.extend(2u16.to_le_bytes()); // block align
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(data.to_le_bytes());
        wav.resize(wav.len() + data as usize, 0);
        wav
    }

    #[test]
    fn decode_resamples_to_the_output_rate_and_channels() {
        // Given half a second of mono 8 kHz audio
        let path = std::env::temp_dir().join(format!("madsister-decode-{}.wav", std::process::id()));
        std::fs::write(&path, silent_wav(0.5)).unwrap();

        // When decoding it for a 48 kHz stereo output
        let samples = decode(&path, 48_000, 2).unwrap();
        std::fs::remove_file(&path).unwrap();

        // Then it has half a second of stereo frames at 48 kHz (give or take the resampler's edges)
        let frames = samples.len() / 2;
        assert!((frames as i64 - 24_000).abs() < 100, "{frames} frames");
    }

    #[test]
    fn decode_of_a_file_that_isnt_audio_fails_with_ffmpegs_reason() {
        // Given
        let path = std::env::temp_dir().join(format!("madsister-decode-{}.txt", std::process::id()));
        std::fs::write(&path, "not audio").unwrap();

        // When / Then
        assert!(decode(&path, 48_000, 2).is_err());
        std::fs::remove_file(&path).unwrap();
    }
}
