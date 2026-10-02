pub mod engine;

use engine::{EngineEvent, Jobs, Packaged};
use sha2::{Digest, Sha256};
use std::io::Read;
use tauri::ipc::{Channel, Response};
use tauri::{Manager, RunEvent, State};

/// The packaged engine; None in dev (`tauri dev`) and when `MADSISTER_ENGINE` overrides it.
struct Engine(Option<Packaged>);

/// Starts `madsister-engine <args>`; returns the job id. Events stream on `on_event` (see `EngineEvent`).
fn start(
    jobs: State<Jobs>,
    engine: State<Engine>,
    args: Vec<String>,
    has_stdin: bool,
    on_event: Channel<EngineEvent>,
) -> Result<u32, String> {
    let mut argv =
        engine::engine_command(std::env::var("MADSISTER_ENGINE").ok(), engine.0.as_ref());
    argv.extend(args);
    spawn(jobs, argv, has_stdin, on_event)
}

fn spawn(
    jobs: State<Jobs>,
    argv: Vec<String>,
    has_stdin: bool,
    on_event: Channel<EngineEvent>,
) -> Result<u32, String> {
    jobs.spawn(&argv, has_stdin, move |event| {
        let _ = on_event.send(event); // the window is gone: nothing to tell
    })
    .map_err(|e| format!("can't start the engine ({}): {e}", argv[0]))
}

#[tauri::command]
fn transcribe(
    jobs: State<Jobs>,
    engine: State<Engine>,
    audio_path: String,
    out_path: String,
    meter: Option<String>,
    sections: bool,
    on_event: Channel<EngineEvent>,
) -> Result<u32, String> {
    let args = engine::transcribe_args(audio_path, out_path, meter, sections);
    start(jobs, engine, args, false, on_event)
}

#[tauri::command]
fn fetch(
    jobs: State<Jobs>,
    engine: State<Engine>,
    url: String,
    out_dir: String,
    on_event: Channel<EngineEvent>,
) -> Result<u32, String> {
    let args = vec!["fetch".into(), url, "--out-dir".into(), out_dir];
    start(jobs, engine, args, false, on_event)
}

/// Records until `stop` (keeps the file) or `cancel` (the file is lost): its stdin is piped for `stop`.
#[tauri::command]
fn record(
    jobs: State<Jobs>,
    engine: State<Engine>,
    out_path: String,
    on_event: Channel<EngineEvent>,
) -> Result<u32, String> {
    if let Some(dir) = std::path::Path::new(&out_path).parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    }
    let args = vec!["record".into(), "--out".into(), out_path];
    start(jobs, engine, args, true, on_event)
}

/// True on a packaged app's first launch, and after an update: `setup_engine` must run before any engine command.
#[tauri::command]
fn engine_needs_setup(engine: State<Engine>) -> bool {
    engine.0.as_ref().is_some_and(Packaged::needs_setup)
}

/// Installs the packaged engine (`setup-engine.sh`): same events as the engine commands. `sections`: also all-in-one
/// (slow: a large download).
#[tauri::command]
fn setup_engine(
    jobs: State<Jobs>,
    engine: State<Engine>,
    sections: bool,
    on_event: Channel<EngineEvent>,
) -> Result<u32, String> {
    let packaged = engine.0.as_ref().ok_or("the engine isn't packaged")?;
    spawn(jobs, packaged.setup_command(sections), false, on_event)
}

/// True when "Detect sections" can run: always in dev or with `MADSISTER_ENGINE`, else once `setup_engine` ran with `sections`.
#[tauri::command]
fn engine_has_sections(engine: State<Engine>) -> bool {
    engine.0.as_ref().is_none_or(Packaged::has_sections)
}

fn packaged_engine(app: &tauri::App) -> Option<Packaged> {
    if tauri::is_dev() || std::env::var_os("MADSISTER_ENGINE").is_some() {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    // The sidecar (bundle.externalBin) is named after the binary, so that madsister and madsister-snapshot can be installed together.
    let uv = exe.with_file_name(format!("{}-uv", exe.file_name()?.to_str()?));
    Some(Packaged {
        uv,
        project: app.path().resource_dir().ok()?.join("engine"),
        env: app.path().app_data_dir().ok()?.join("engine-env"),
        version: app.package_info().version.to_string(),
    })
}

#[tauri::command]
fn stop(jobs: State<Jobs>, job_id: u32) {
    jobs.stop(job_id);
}

#[tauri::command]
fn cancel(jobs: State<Jobs>, job_id: u32) {
    jobs.cancel(job_id);
}

/// The file's hex sha256 (the Song's `audio.sha256`, spec §4), streamed; None when it can't be read (missing, moved).
/// Runs off the main thread: songs open first.
#[tauri::command(async)]
fn audio_sha256(path: String) -> Option<String> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        match file.read(&mut buf).ok()? {
            0 => break,
            n => hasher.update(&buf[..n]),
        }
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

/// The audio file's bytes, played as a blob in the webview (WebKitGTK's GStreamer can't read the asset protocol).
/// It reads outside the fs scope, so only audio files (the extensions of the app's audio dialogs).
#[tauri::command(async)]
fn read_audio(path: String) -> Result<Response, String> {
    let is_audio = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| {
            ["mp3", "wav", "flac", "m4a", "ogg"].contains(&e.to_ascii_lowercase().as_str())
        });
    if !is_audio {
        return Err(format!("not an audio file: {path}"));
    }
    std::fs::read(&path)
        .map(Response::new)
        .map_err(|e| format!("can't read {path}: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Jobs::default())
        .setup(|app| {
            app.manage(Engine(packaged_engine(app)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            transcribe,
            fetch,
            record,
            stop,
            cancel,
            engine_needs_setup,
            setup_engine,
            engine_has_sections,
            audio_sha256,
            read_audio
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<Jobs>().cancel_all();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_sha256_of_known_content_returns_its_hex_digest() {
        // Given a file containing "abc"
        let path = std::env::temp_dir().join(format!("madsister-sha256-{}", std::process::id()));
        std::fs::write(&path, "abc").unwrap();

        // When
        let digest = audio_sha256(path.display().to_string());
        std::fs::remove_file(&path).unwrap();

        // Then the FIPS 180-2 test vector
        assert_eq!(
            digest.as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        // And a missing file has none
        assert_eq!(audio_sha256(path.display().to_string()), None);
    }

    #[test]
    fn read_audio_reads_audio_files_only() {
        // Given an audio file and a text file
        let dir = std::env::temp_dir();
        let song = dir.join(format!("madsister-read-{}.MP3", std::process::id()));
        let text = dir.join(format!("madsister-read-{}.txt", std::process::id()));
        std::fs::write(&song, "ID3").unwrap();
        std::fs::write(&text, "secret").unwrap();

        // When
        let read_song = read_audio(song.display().to_string());
        let read_text = read_audio(text.display().to_string());
        std::fs::remove_file(&song).unwrap();
        std::fs::remove_file(&text).unwrap();

        // Then only the audio file is read, and a missing one is an error
        assert!(read_song.is_ok());
        assert!(read_text.is_err_and(|e| e.starts_with("not an audio file")));
        assert!(read_audio(song.display().to_string()).is_err());
    }
}
