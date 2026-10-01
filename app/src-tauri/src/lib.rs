pub mod engine;

use engine::{EngineEvent, Jobs};
use tauri::ipc::Channel;
use tauri::{Manager, RunEvent, State};

/// Starts `madsister-engine transcribe`; returns the job id. Events stream on `on_event` (see `EngineEvent`).
#[tauri::command]
fn transcribe(
    jobs: State<Jobs>,
    audio_path: String,
    out_path: String,
    meter: Option<u8>,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Jobs::default())
        .invoke_handler(tauri::generate_handler![transcribe, cancel])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<Jobs>().cancel_all();
            }
        });
}
