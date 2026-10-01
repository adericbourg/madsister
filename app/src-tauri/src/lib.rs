pub mod engine;

use engine::{EngineEvent, Jobs};
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use tauri::{Manager, RunEvent, State};

/// Starts `madsister-engine transcribe`; returns the job id. Events stream on `on_event` (see `EngineEvent`).
#[tauri::command]
fn transcribe(
    jobs: State<Jobs>,
    audio_path: String,
    out_path: String,
    meter: Option<String>,
    sections: bool,
    on_event: Channel<EngineEvent>,
) -> Result<u32, String> {
    let mut argv = engine::engine_command();
    argv.extend(engine::transcribe_args(
        audio_path, out_path, meter, sections,
    ));
    jobs.spawn(&argv, move |event| {
        let _ = on_event.send(event); // the window is gone: nothing to tell
    })
    .map_err(|e| format!("can't start the engine ({}): {e}", argv[0]))
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
    std::io::copy(&mut std::fs::File::open(path).ok()?, &mut hasher).ok()?;
    Some(format!("{:x}", hasher.finalize()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Jobs::default())
        .invoke_handler(tauri::generate_handler![transcribe, cancel, audio_sha256])
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
}
