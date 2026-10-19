use super::*;

const SOURCES: &str = "/data/sources";

fn installed(_: &str) -> bool {
    true
}

fn nothing_exists(_: &Path) -> bool {
    false
}

fn run(importer: &mut Importer) -> Next {
    importer.run(Path::new(SOURCES), "Recording x.wav", installed, nothing_exists)
}

fn progress(stage: &str, pct: u8, elapsed_sec: Option<u32>) -> EngineEvent {
    EngineEvent::Progress { stage: stage.into(), pct, elapsed_sec }
}

fn result(path: &str) -> EngineEvent {
    EngineEvent::Result { path: path.into() }
}

fn transcribe(audio: &str, out: &str, meter: Option<&str>, sections: bool, mode: &str) -> Next {
    Next::Start(Request::Transcribe {
        audio: audio.into(),
        out: out.into(),
        meter: meter.map(str::to_string),
        sections,
        mode: mode.into(),
    })
}

#[test]
fn a_dropped_file_waits_for_run_then_shows_progress_and_hands_over_the_result() {
    // Given a dropped mp3
    let mut importer = Importer::default();
    importer.select_file(Path::new("/music/song.mp3"));
    assert!(importer.can_run() && !importer.is_running());

    // When choosing the options then running
    importer.detect_sections = true;
    importer.meter = Some(ForcedMeter::SixEight);
    let next = run(&mut importer);

    // Then the engine starts on it, writing next to it, with that meter and all-in-one
    assert_eq!(next, transcribe("/music/song.mp3", "/music/song.madsister.json", Some("6/8"), true, "fast"));
    assert_eq!(importer.status(), "Starting…");
    assert!(!importer.can_run());

    // When the engine starts and reports progress, Then the stage follows
    importer.started(Ok(42));
    importer.event(progress("chords", 60, None), installed, nothing_exists);
    assert_eq!((importer.status(), importer.job.as_ref().map(|j| j.pct)), ("chords".into(), Some(60)));

    // When the engine reports its result, Then the song is opened and the job is over
    let next = importer.event(result("/music/song.madsister.json"), installed, nothing_exists);
    assert_eq!(next, Next::Open("/music/song.madsister.json".into()));
    assert!(!importer.is_running());
}

#[test]
fn missing_features_are_installed_before_transcribing() {
    // Given sections and accurate wanted, neither installed
    let mut importer = Importer { detect_sections: true, accurate: true, ..Importer::default() };
    importer.select_file(Path::new("/music/song.mp3"));

    // When running, Then one install of both runs first
    let next = importer.run(Path::new(SOURCES), "r.wav", |_| false, nothing_exists);
    assert_eq!(next, Next::Start(Request::Setup(vec!["sections".into(), "separate".into()])));

    // When the install ends, Then the transcription starts with all-in-one, in accurate mode
    importer.started(Ok(9));
    let next = importer.event(result("/models"), installed, nothing_exists);
    assert_eq!(next, transcribe("/music/song.mp3", "/music/song.madsister.json", None, true, "accurate"));
}

#[test]
fn an_install_that_doesnt_stick_still_transcribes_once() {
    // Given an install of sections that the engine never records as installed
    let mut importer = Importer { detect_sections: true, ..Importer::default() };
    importer.select_file(Path::new("/music/song.mp3"));
    importer.run(Path::new(SOURCES), "r.wav", |_| false, nothing_exists);

    // When it ends, Then the transcription starts instead of another install
    let next = importer.event(result("/models"), |_| false, nothing_exists);
    assert_eq!(next, transcribe("/music/song.mp3", "/music/song.madsister.json", None, true, "fast"));
}

#[test]
fn a_cancelled_job_ends_without_a_result_or_an_error() {
    // Given a running transcription
    let mut importer = Importer::default();
    importer.select_file(Path::new("/music/take.wav"));
    run(&mut importer);
    importer.started(Ok(42));

    // When it's cancelled, Then the job to kill is known and the importer is idle again
    assert_eq!(importer.job_id(), Some(42));
    assert_eq!(importer.event(EngineEvent::Cancelled, installed, nothing_exists), Next::Nothing);
    assert!(!importer.is_running());
    assert_eq!(importer.failure, None);
}

#[test]
fn an_engine_failure_shows_its_message_and_stderr() {
    // Given a running transcription
    let mut importer = Importer::default();
    importer.select_file(Path::new("/music/song.flac"));
    run(&mut importer);

    // When the engine fails, Then its message and output are kept
    let error = EngineEvent::Error { message: "no beats installed".into(), stderr: "Traceback: boom".into() };
    importer.event(error, installed, nothing_exists);
    assert_eq!(importer.failure, Some(Failure { message: "no beats installed".into(), stderr: "Traceback: boom".into() }));
    assert!(!importer.is_running());

    // When the engine can't even start, Then that is shown too
    run(&mut importer);
    importer.started(Err("can't start the engine (uv): not found".into()));
    assert!(importer.failure.as_ref().is_some_and(|f| f.message.starts_with("can't start the engine")));
    assert!(!importer.is_running());
}

#[test]
fn unsupported_files_and_declined_overwrites_dont_start() {
    // Given a text file, Then it is refused and there's nothing to run
    let mut importer = Importer::default();
    importer.select_file(Path::new("/music/notes.txt"));
    assert!(importer.failure.as_ref().is_some_and(|f| f.message.contains("notes.txt") && f.message.contains("mp3, wav, flac, m4a or ogg")));
    assert!(!importer.can_run());

    // When an mp3's song file exists, Then running asks about that file first
    importer.select_file(Path::new("/music/song.MP3"));
    let next = importer.run(Path::new(SOURCES), "r.wav", installed, |_| true);
    assert_eq!(next, Next::ConfirmOverwrite("/music/song.madsister.json".into()));
    assert!(!importer.is_running());

    // When the answer is yes, Then it transcribes
    assert_eq!(
        importer.transcribe_confirmed(installed),
        transcribe("/music/song.MP3", "/music/song.madsister.json", None, false, "fast")
    );
}

#[test]
fn a_url_is_downloaded_then_transcribed_next_to_the_file() {
    // Given a URL
    let mut importer = Importer { source: Source::Url, url: "https://example.org/v?id=1".into(), ..Importer::default() };

    // When running, Then it is downloaded into the sources folder
    assert_eq!(run(&mut importer), Next::Start(Request::Fetch { url: "https://example.org/v?id=1".into(), out_dir: SOURCES.into() }));
    importer.started(Ok(7));
    importer.event(progress("download", 30, None), installed, nothing_exists);
    assert_eq!(importer.status(), "download");

    // When the download ends, Then the file (any format) is transcribed next to it
    let next = importer.event(result("/data/sources/My song.webm"), installed, nothing_exists);
    assert_eq!(next, transcribe("/data/sources/My song.webm", "/data/sources/My song.madsister.json", None, false, "fast"));
}

#[test]
fn a_non_http_url_is_refused() {
    // Given / When
    let mut importer = Importer { source: Source::Url, url: "file:///etc/passwd".into(), ..Importer::default() };

    // Then
    assert_eq!(run(&mut importer), Next::Nothing);
    assert!(importer.failure.as_ref().is_some_and(|f| f.message.contains("http")));
}

#[test]
fn a_recording_shows_its_time_then_is_transcribed_once_stopped() {
    // Given a recording
    let mut importer = Importer { source: Source::Record, ..Importer::default() };
    assert_eq!(run(&mut importer), Next::Start(Request::Record { out: "/data/sources/Recording x.wav".into() }));
    importer.started(Ok(8));

    // When 65 s are recorded, Then the time shows
    importer.event(progress("record", 0, Some(65)), installed, nothing_exists);
    assert_eq!(importer.status(), "Recording 1:05");
    assert!(importer.is_recording());

    // When stopped, the engine writes the file: Then it is transcribed
    let next = importer.event(result("/data/sources/Recording x.wav"), installed, nothing_exists);
    assert_eq!(next, transcribe("/data/sources/Recording x.wav", "/data/sources/Recording x.madsister.json", None, false, "fast"));
}

#[test]
fn recording_name_writes_the_utc_date_and_time() {
    // Given / When / Then
    assert_eq!(recording_name(0), "Recording 1970-01-01T00-00-00.wav");
    assert_eq!(recording_name(1_792_412_205), "Recording 2026-10-19T12-16-45.wav");
    assert_eq!(recording_name(951_782_400), "Recording 2000-02-29T00-00-00.wav");
}

#[test]
fn the_engine_setup_shows_its_stages_then_is_done() {
    // Given a first launch
    let mut setup = EngineSetup { is_needed: true, ..Default::default() };

    // When it starts, Then it installs first, and a second run while it runs starts nothing
    assert!(setup.run());
    assert_eq!(setup.status(), "Installing Python and the engine's libraries…");
    assert!(!setup.run());

    // When the models download, Then
    setup.event(progress("setup", 50, None));
    assert_eq!((setup.status(), setup.progress.as_ref().map(|p| p.1)), ("Downloading the models…", Some(50)));

    // When it ends, Then the engine is ready
    setup.event(result("/models"));
    assert!(!setup.is_needed && setup.progress.is_none() && setup.failure.is_none());
}

#[test]
fn a_failed_engine_setup_keeps_the_engine_output_until_retried() {
    // Given a running setup
    let mut setup = EngineSetup { is_needed: true, ..Default::default() };
    setup.run();

    // When it fails, Then the message and output are kept, and nothing runs
    setup.event(EngineEvent::Error { message: "no network".into(), stderr: "uv: dns error".into() });
    assert_eq!(setup.failure, Some(Failure { message: "no network".into(), stderr: "uv: dns error".into() }));
    assert!(setup.is_needed && setup.progress.is_none() && setup.status().is_empty());

    // When retrying, Then it runs again without the failure
    assert!(setup.run());
    assert_eq!(setup.failure, None);

    // When it's cancelled (the app closes), Then it says so
    setup.event(EngineEvent::Cancelled);
    assert_eq!(setup.failure.map(|f| f.message), Some("The setup was cancelled.".into()));
}
