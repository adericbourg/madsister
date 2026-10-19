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

/// Decoding stops there: a song file names its audio, so a huge or crafted one mustn't exhaust the memory.
const MAX_SECONDS: u32 = 30 * 60;

/// The file's samples, interleaved, at `rate` Hz and `channels` channels; at most `MAX_SECONDS` of them.
pub fn decode(path: &Path, rate: u32, channels: u16) -> Result<Vec<f32>, String> {
    // A song file can come from someone else: ffmpeg only opens local files, never a URL or a concat/playlist input.
    let mut input = std::ffi::OsString::from("file:");
    input.push(path);
    let mut command = Command::new(ffmpeg());
    command
        .args(["-nostdin", "-loglevel", "error", "-protocol_whitelist", "file", "-i"])
        .arg(input)
        .args(["-t", &MAX_SECONDS.to_string()])
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

/// The song's audio file: a relative `audio.path` is relative to the song file's folder.
pub fn audio_path(song_path: &Path, audio: &str) -> PathBuf {
    let audio = Path::new(audio);
    if audio.is_absolute() { audio.to_path_buf() } else { song_path.parent().unwrap_or(Path::new("")).join(audio) }
}

/// The inverse of `audio_path`: `audio` relative to the song file's folder, so the song and its audio can move together.
/// Stays as is when the paths share no root (another drive).
/// shortcut: the comparison is case-sensitive, so Windows paths differing only by case stay absolute; normalise if that bites.
pub fn relative_audio_path(song_path: &Path, audio: &Path) -> String {
    let (song, audio) = (song_path.to_string_lossy(), audio.to_string_lossy());
    let song: Vec<&str> = song.split(['/', '\\']).collect();
    let song = &song[..song.len() - 1];
    let parts: Vec<&str> = audio.split(['/', '\\']).collect();
    let mut common = 0;
    while common < song.len() && common < parts.len() - 1 && song[common] == parts[common] {
        common += 1;
    }
    if common == 0 {
        return audio.into_owned();
    }
    let ups = song[common..].iter().map(|_| "..");
    ups.chain(parts[common..].iter().copied()).collect::<Vec<_>>().join("/")
}

/// The file's sha256 in hex, read in chunks; None when it can't be read.
pub fn sha256(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        match file.read(&mut buf).ok()? {
            0 => break,
            n => hasher.update(&buf[..n]),
        }
    }
    Some(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// What's wrong with the song's audio file, if anything.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileCheck {
    /// It can't be read: the song can't be played until it's located.
    Missing,
    /// It isn't the file the song was transcribed from.
    Changed,
}

/// The audio file's state from the sha256 the song expects and the file's own (None: the file can't be read).
pub fn check_file(expected: &str, actual: Option<&str>) -> Option<FileCheck> {
    match actual {
        None => Some(FileCheck::Missing),
        Some(sha256) if sha256 != expected => Some(FileCheck::Changed),
        Some(_) => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Loading,
    Ready,
    Failed(String),
}

struct Shared {
    // shortcut: the whole song decoded in memory (~115 MB for 5 minutes at 48 kHz stereo, at most MAX_SECONDS); decode
    // in chunks while playing if long recordings become common.
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
    fn decode_only_opens_local_files() {
        // Given a playable WAV reached through ffmpeg's concat protocol, which a bare ffmpeg would decode
        let wav = std::env::temp_dir().join(format!("madsister-concat-{}.wav", std::process::id()));
        std::fs::write(&wav, silent_wav(0.1)).unwrap();
        let concat = format!("concat:{}", wav.display());

        // When decoding it
        let decoded = decode(Path::new(&concat), 48_000, 2);
        std::fs::remove_file(&wav).unwrap();

        // Then it is read as a local file named "concat:…", which doesn't exist
        assert!(decoded.is_err());
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

    #[test]
    fn audio_path_resolves_relative_paths_against_the_song_folder_and_keeps_absolute_ones() {
        // Given a relative audio path, Then it is resolved next to the song
        assert_eq!(audio_path(Path::new("/music/s.madsister.json"), "a.mp3"), Path::new("/music/a.mp3"));
        // And absolute paths are left alone
        assert_eq!(audio_path(Path::new("/music/s.madsister.json"), "/other/a.mp3"), Path::new("/other/a.mp3"));
    }

    #[test]
    fn relative_audio_path_is_relative_to_the_song_folder_and_round_trips_with_audio_path() {
        let relative = |song: &str, audio: &str| relative_audio_path(Path::new(song), Path::new(audio));
        // Given a song file, When the audio is next to it, below it or above it, Then the path is relative
        assert_eq!(relative("/music/s.madsister.json", "/music/a.mp3"), "a.mp3");
        assert_eq!(relative("/music/s.madsister.json", "/music/live/a.mp3"), "live/a.mp3");
        assert_eq!(relative("/music/songs/s.madsister.json", "/music/live/a.mp3"), "../live/a.mp3");
        assert_eq!(relative("C:\\music\\s.madsister.json", "C:\\music\\live\\a.mp3"), "live/a.mp3");
        // And resolving it gives back the same file
        let back = audio_path(Path::new("/music/songs/s.madsister.json"), &relative("/music/songs/s.madsister.json", "/music/live/a.mp3"));
        assert_eq!(back, Path::new("/music/songs/../live/a.mp3"));
        // And another drive keeps the absolute path
        assert_eq!(relative("C:\\music\\s.madsister.json", "D:\\a.mp3"), "D:\\a.mp3");
    }

    #[test]
    fn sha256_streams_the_file_and_is_none_when_it_cant_be_read() {
        // Given a file bigger than one read
        let path = std::env::temp_dir().join(format!("madsister-sha-{}.bin", std::process::id()));
        std::fs::write(&path, vec![b'a'; 100_000]).unwrap();

        // When hashing it, and a missing file
        let sha = sha256(&path);
        std::fs::remove_file(&path).unwrap();

        // Then (from `head -c 100000 /dev/zero | tr '\\0' a | sha256sum`)
        assert_eq!(sha.as_deref(), Some("6d1cf22d7cc09b085dfc25ee1a1f3ae0265804c607bc2074ad253bcc82fd81ee"));
        assert_eq!(sha256(Path::new("/nonexistent/a.mp3")), None);
    }

    #[test]
    fn check_file_tells_a_missing_file_from_a_changed_one() {
        assert_eq!(check_file("abc", None), Some(FileCheck::Missing));
        assert_eq!(check_file("abc", Some("def")), Some(FileCheck::Changed));
        assert_eq!(check_file("abc", Some("abc")), None);
    }
}
