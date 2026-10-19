//! Audio import (spec F-IN-1..5, NF-4): a file (dialog or drop), a URL or the microphone, the options, then engine jobs
//! with progress and cancel. Pure: the UI starts the jobs it asks for and feeds their events back.

use std::path::{Path, PathBuf};

use crate::engine::{EngineEvent, FEATURES};

pub const AUDIO_EXTENSIONS: [&str; 6] = ["mp3", "wav", "flac", "m4a", "ogg", "webm"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Source {
    #[default]
    File,
    Url,
    Record,
}

/// A forced meter, as `madsister-engine transcribe --meter` takes it (F-IN-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcedMeter {
    Four,
    Three,
    SixEight,
}

impl ForcedMeter {
    fn arg(self) -> &'static str {
        match self {
            ForcedMeter::Four => "4",
            ForcedMeter::Three => "3",
            ForcedMeter::SixEight => "6/8",
        }
    }
}

/// An engine job to start.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Installs these of `FEATURES` (large downloads, once).
    Setup(Vec<String>),
    Transcribe { audio: PathBuf, out: PathBuf, meter: Option<String>, sections: bool, mode: String },
    Fetch { url: String, out_dir: PathBuf },
    Record { out: PathBuf },
}

/// What the UI does next.
#[derive(Debug, Clone, PartialEq)]
pub enum Next {
    Nothing,
    Start(Request),
    /// Asks whether to overwrite this song file; yes goes to `transcribe_confirmed`.
    ConfirmOverwrite(PathBuf),
    /// The transcription wrote this song file.
    Open(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Setup,
    Transcribe,
    Fetch,
    Record,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    kind: Kind,
    /// Known once the job has started.
    pub id: Option<u32>,
    pub stage: String,
    pub pct: u8,
    pub elapsed_sec: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Failure {
    pub message: String,
    pub stderr: String,
}

#[derive(Debug, Default)]
pub struct Importer {
    pub source: Source,
    pub audio_path: Option<PathBuf>,
    pub url: String,
    pub meter: Option<ForcedMeter>,
    pub detect_sections: bool,
    pub accurate: bool,
    pub job: Option<Job>,
    pub failure: Option<Failure>,
    /// The audio waiting for an install or an overwrite answer.
    waiting: Option<PathBuf>,
}

/// `dir/song.mp3` → `dir/song.madsister.json`: the song goes next to its audio.
pub fn song_path_for(audio: &Path) -> PathBuf {
    audio.with_extension("madsister.json")
}

fn fail(message: impl Into<String>) -> Failure {
    Failure { message: message.into(), stderr: String::new() }
}

impl Importer {
    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    /// Run is possible: nothing running, and the source has what it needs.
    pub fn can_run(&self) -> bool {
        !self.is_running()
            && match self.source {
                Source::File => self.audio_path.is_some(),
                Source::Url => !self.url.trim().is_empty(),
                Source::Record => true,
            }
    }

    /// A file from the dialog or dropped on the window; other formats are refused.
    pub fn select_file(&mut self, path: &Path) {
        let extension = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
        if extension.is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.as_str())) {
            self.failure = None;
            self.audio_path = Some(path.to_path_buf());
            self.source = Source::File;
        } else {
            self.failure = Some(fail(format!("Can't import {}: use an mp3, wav, flac, m4a or ogg file.", path.display())));
        }
    }

    /// The Run button. `sources` is where downloads and recordings go; `recording_name` names a recording.
    pub fn run(&mut self, sources: &Path, recording_name: &str, has: impl Fn(&str) -> bool, exists: impl Fn(&Path) -> bool) -> Next {
        self.failure = None;
        match self.source {
            Source::File => match self.audio_path.clone() {
                Some(audio) => self.transcribe_file(audio, has, exists, false),
                None => Next::Nothing,
            },
            Source::Url => {
                let url = self.url.trim();
                if !(url.starts_with("http://") || url.starts_with("https://")) {
                    self.failure = Some(fail(format!("Can't fetch {url}: use an http(s) URL.")));
                    return Next::Nothing;
                }
                self.start(Request::Fetch { url: url.into(), out_dir: sources.into() })
            }
            Source::Record => self.start(Request::Record { out: sources.join(recording_name) }),
        }
    }

    /// The overwrite question was answered yes.
    pub fn transcribe_confirmed(&mut self, has: impl Fn(&str) -> bool) -> Next {
        match self.waiting.take() {
            Some(audio) => self.transcribe_file(audio, has, |_| false, true),
            None => Next::Nothing,
        }
    }

    /// Transcribes into the song file next to the audio, after installing what the options need.
    fn transcribe_file(&mut self, audio: PathBuf, has: impl Fn(&str) -> bool, exists: impl Fn(&Path) -> bool, is_confirmed: bool) -> Next {
        let wanted = FEATURES.iter().filter(|&&f| if f == "sections" { self.detect_sections } else { self.accurate });
        let missing: Vec<String> = wanted.filter(|f| !has(f)).map(|f| f.to_string()).collect();
        if !missing.is_empty() {
            self.waiting = Some(audio);
            return self.start(Request::Setup(missing));
        }
        let out = song_path_for(&audio);
        if !is_confirmed && exists(&out) {
            self.waiting = Some(audio);
            return Next::ConfirmOverwrite(out);
        }
        let mode = if self.accurate { "accurate" } else { "fast" };
        self.start(Request::Transcribe {
            audio,
            out,
            meter: self.meter.map(|m| m.arg().to_string()),
            sections: self.detect_sections,
            mode: mode.into(),
        })
    }

    fn start(&mut self, request: Request) -> Next {
        let kind = match request {
            Request::Setup(_) => Kind::Setup,
            Request::Transcribe { .. } => Kind::Transcribe,
            Request::Fetch { .. } => Kind::Fetch,
            Request::Record { .. } => Kind::Record,
        };
        self.job = Some(Job { kind, id: None, stage: "Starting…".into(), pct: 0, elapsed_sec: None });
        Next::Start(request)
    }

    /// The job started with this id; or it couldn't start.
    pub fn started(&mut self, result: Result<u32, String>) {
        match result {
            Ok(id) => {
                if let Some(job) = &mut self.job {
                    job.id = Some(id);
                }
            }
            Err(e) => {
                self.job = None;
                self.failure = Some(fail(e));
            }
        }
    }

    /// An event of the running job.
    pub fn event(&mut self, event: EngineEvent, has: impl Fn(&str) -> bool, exists: impl Fn(&Path) -> bool) -> Next {
        let Some(job) = &mut self.job else { return Next::Nothing };
        match event {
            EngineEvent::Progress { stage, pct, elapsed_sec } => {
                job.stage = stage;
                job.pct = pct;
                job.elapsed_sec = elapsed_sec;
                Next::Nothing
            }
            EngineEvent::Result { path } => {
                let kind = job.kind;
                self.job = None;
                match kind {
                    Kind::Transcribe => Next::Open(path.into()),
                    Kind::Fetch | Kind::Record => self.transcribe_file(path.into(), has, exists, false),
                    // Not checked again: an install that didn't stick would start another one, endlessly. If a feature is
                    // still missing, the engine says so.
                    Kind::Setup => match self.waiting.take() {
                        Some(audio) => self.transcribe_file(audio, |_| true, exists, false),
                        None => Next::Nothing,
                    },
                }
            }
            EngineEvent::Error { message, stderr } => {
                self.job = None;
                self.failure = Some(Failure { message, stderr });
                Next::Nothing
            }
            EngineEvent::Cancelled => {
                self.job = None;
                Next::Nothing
            }
        }
    }

    /// The running job's id, for Cancel (any job) or Stop (a recording, which keeps its file).
    pub fn job_id(&self) -> Option<u32> {
        self.job.as_ref().and_then(|j| j.id)
    }

    pub fn is_recording(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.kind == Kind::Record)
    }

    /// What the status line says.
    pub fn status(&self) -> String {
        match &self.job {
            Some(job) if job.kind == Kind::Record => {
                let elapsed = job.elapsed_sec.unwrap_or(0);
                format!("Recording {}:{:02}", elapsed / 60, elapsed % 60)
            }
            Some(job) => job.stage.clone(),
            None => String::new(),
        }
    }
}

/// A packaged app's first-launch engine install (M6-2), which the importer waits for. One for the whole window: a second
/// setup would race the first on the same env.
#[derive(Debug, Default)]
pub struct EngineSetup {
    pub is_needed: bool,
    /// The running setup's stage and percent.
    pub progress: Option<(String, u8)>,
    pub failure: Option<Failure>,
}

impl EngineSetup {
    /// Starts the setup unless one runs: true when the caller must start its job.
    pub fn run(&mut self) -> bool {
        if self.progress.is_some() {
            return false;
        }
        self.failure = None;
        self.progress = Some(("install".into(), 0));
        true
    }

    /// An event of the setup's job (an `Error` too when it couldn't start).
    pub fn event(&mut self, event: EngineEvent) {
        self.progress = None;
        match event {
            EngineEvent::Progress { stage, pct, .. } => self.progress = Some((stage, pct)),
            EngineEvent::Result { .. } => self.is_needed = false,
            EngineEvent::Error { message, stderr } => self.failure = Some(Failure { message, stderr }),
            EngineEvent::Cancelled => self.failure = Some(fail("The setup was cancelled.")),
        }
    }

    /// What the status line says.
    pub fn status(&self) -> &str {
        match self.progress.as_ref().map(|(stage, _)| stage.as_str()) {
            Some("install") => "Installing Python and the engine's libraries…",
            Some("setup") => "Downloading the models…",
            Some(stage) => stage,
            None => "",
        }
    }
}

/// "Recording 2026-10-19T12-30-05.wav" for a time in seconds since 1970 (UTC), as the TypeScript app named them.
pub fn recording_name(unix_sec: u64) -> String {
    let (days, secs) = ((unix_sec / 86_400) as i64, unix_sec % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("Recording {year:04}-{month:02}-{day:02}T{:02}-{:02}-{:02}.wav", secs / 3600, secs % 3600 / 60, secs % 60)
}

#[cfg(test)]
mod tests;
