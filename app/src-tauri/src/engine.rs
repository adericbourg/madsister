// Engine bridge (spec §3.2, F-IN-4): spawns `madsister-engine`, forwards its JSON Lines, kills it on cancel.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt; // ponytail: unix only (Linux, macOS), Windows is out of scope (D4)
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// Engine lines, plus `cancelled` and the stderr tail on errors. Exactly one of result/error/cancelled ends a job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EngineEvent {
    Progress {
        stage: String,
        pct: u8,
        /// `record` only: whole seconds recorded.
        #[serde(
            rename = "elapsedSec",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        elapsed_sec: Option<u32>,
    },
    Result {
        path: String,
    },
    Error {
        message: String,
        #[serde(default)]
        stderr: String,
    },
    Cancelled,
}

/// The engine installed by `setup-engine.py` in a packaged app (`tauri build`): `uv` is a sidecar next to the app's
/// binary, the `engine/` project a resource, the env lives in the app data dir.
pub struct Packaged {
    pub uv: PathBuf,
    pub project: PathBuf,
    pub env: PathBuf,
    pub version: String,
}

const SETUP_SCRIPT: &str = include_str!("../setup-engine.py");

/// What `setup-engine.py` installs on demand: `sections` (all-in-one), `separate` (Demucs, accurate mode).
pub const FEATURES: [&str; 2] = ["sections", "separate"];

impl Packaged {
    /// No env yet, or one set up by another app version.
    pub fn needs_setup(&self) -> bool {
        std::fs::read_to_string(self.env.join(".madsister-version")).ok()
            != Some(self.version.clone())
    }

    /// One of `FEATURES` is installed for this app version.
    pub fn has(&self, feature: &str) -> bool {
        std::fs::read_to_string(self.env.join(format!(".madsister-{feature}"))).ok()
            == Some(self.version.clone())
    }

    /// Installs the env with the `requested` features plus the ones already installed for this version (a sync is exact).
    pub fn setup_command(&self, requested: &[String]) -> Vec<String> {
        let path = |p: &PathBuf| p.display().to_string();
        // uv's managed Python runs it: there's no Python before the env, nor sh on Windows.
        let mut command = vec![path(&self.uv)];
        command.extend(
            ["run", "--no-project", "--managed-python", "--python", "3.11", "python", "-c"]
                .map(String::from),
        );
        command.extend([
            SETUP_SCRIPT.into(),
            path(&self.uv),
            path(&self.project),
            path(&self.env),
            self.version.clone(),
        ]);
        command.extend(
            FEATURES
                .into_iter()
                .filter(|f| requested.iter().any(|r| r == f) || self.has(f))
                .map(String::from),
        );
        command
    }
}

/// `MADSISTER_ENGINE` (a command line, split on whitespace), then the packaged engine, then the dev engine.
/// `--no-sync`: a plain `uv run` would re-sync the env and remove the model dependency groups.
pub fn engine_command(
    override_command: Option<String>,
    packaged: Option<&Packaged>,
) -> Vec<String> {
    match (override_command, packaged) {
        (Some(cmd), _) if !cmd.trim().is_empty() => {
            cmd.split_whitespace().map(String::from).collect()
        }
        (_, Some(packaged)) => vec![venv_engine(&packaged.env).display().to_string()],
        _ => [
            "uv",
            "run",
            "--project",
            DEV_PROJECT,
            "--no-sync",
            "madsister-engine",
        ]
        .map(String::from)
        .to_vec(),
    }
}

const DEV_PROJECT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../engine");

/// The engine's launcher in a venv: `Scripts\madsister-engine.exe` on Windows.
fn venv_engine(venv: &Path) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts").join("madsister-engine.exe")
    } else {
        venv.join("bin").join("madsister-engine")
    }
}

/// How to install the dev engine when `command` is it and it's missing: `uv run --no-sync` never installs it.
pub fn missing_dev_engine(command: &[String]) -> Option<String> {
    let installed = venv_engine(&Path::new(DEV_PROJECT).join(".venv")).exists();
    (command == engine_command(None, None) && !installed).then(|| {
        "the dev engine isn't installed: run `cd engine && uv sync --group beats-madmom --group chords-cnnlstm \
         --group beats-allinone --group fetch --group record --group separate` (see CONTRIBUTING.md)"
            .into()
    })
}

/// `transcribe` arguments after the engine command. `sections`: all-in-one beats + sections (slow opt-in, ~18× madmom).
/// `mode`: `fast` or `accurate` (D7), checked by the engine.
pub fn transcribe_args(
    audio_path: String,
    out_path: String,
    meter: Option<String>,
    sections: bool,
    mode: String,
) -> Vec<String> {
    let mut args = vec!["transcribe".into(), audio_path, "--out".into(), out_path, "--mode".into(), mode];
    if let Some(meter) = meter {
        args.extend(["--meter".into(), meter]);
    }
    if sections {
        args.extend(["--beats".into(), "allinone".into()]);
    }
    args
}

const TAIL_LINES: usize = 20;

fn push_bounded(tail: &mut VecDeque<String>, line: String) {
    if tail.len() == TAIL_LINES {
        tail.pop_front();
    }
    tail.push_back(line);
}

fn last_lines(reader: impl Read) -> VecDeque<String> {
    let mut tail = VecDeque::new();
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        push_bounded(&mut tail, line);
    }
    tail
}

/// The running jobs by pid (a job id is its child's pid, which is also its process group id), with their stdin when piped.
#[derive(Clone, Default)]
pub struct Jobs(Arc<Mutex<HashMap<u32, Option<ChildStdin>>>>);

impl Jobs {
    pub fn spawn(
        &self,
        argv: &[String],
        has_stdin: bool,
        emit: impl Fn(EngineEvent) + Send + 'static,
    ) -> io::Result<u32> {
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            // A null stdin is an EOF, which stops `record` at once.
            .stdin(if has_stdin {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0) // its own group, so cancel also reaches uv's child and ffmpeg/Demucs
            .spawn()?;
        let pid = child.id();
        self.0.lock().unwrap().insert(pid, child.stdin.take());
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let stderr_reader = thread::spawn(move || last_lines(stderr));
        let jobs = self.clone();
        thread::spawn(move || {
            let mut last = None;
            let mut stray = VecDeque::new();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                match serde_json::from_str(&line) {
                    Ok(event @ EngineEvent::Progress { .. }) => emit(event),
                    Ok(event) => last = Some(event),
                    Err(_) => push_bounded(&mut stray, line),
                }
            }
            stray.extend(stderr_reader.join().unwrap_or_default());
            let stderr = Vec::from(stray).join("\n");
            // Leave the set before reaping: cancel never signals a pid that may have been reused.
            let was_cancelled = jobs.0.lock().unwrap().remove(&pid).is_none();
            let status = child.wait();
            let is_success = status.as_ref().is_ok_and(|s| s.success());
            emit(match last {
                _ if was_cancelled => EngineEvent::Cancelled,
                Some(EngineEvent::Error { message, .. }) => EngineEvent::Error { message, stderr },
                Some(result @ EngineEvent::Result { .. }) if is_success => result,
                _ => EngineEvent::Error {
                    message: format!(
                        "the engine stopped without a result ({})",
                        status.map_or_else(|e| e.to_string(), |s| s.to_string())
                    ),
                    stderr,
                },
            });
        });
        Ok(pid)
    }

    /// Asks a job spawned with a stdin to stop gracefully (`record` writes its file, then its result). No-op otherwise.
    pub fn stop(&self, job_id: u32) {
        if let Some(Some(stdin)) = self.0.lock().unwrap().get_mut(&job_id) {
            let _ = stdin.write_all(b"stop\n"); // it has just ended: its own terminal event says so
        }
    }

    /// Kills the job's whole process group. No-op once the job has ended.
    pub fn cancel(&self, job_id: u32) {
        let mut live = self.0.lock().unwrap();
        if live.remove(&job_id).is_some() {
            // SAFETY: plain syscall; the group exists since its leader isn't reaped while it's in the set.
            unsafe { libc::killpg(job_id as libc::pid_t, libc::SIGKILL) };
        }
    }

    /// Called on app exit: the jobs are in their own process groups, so they'd outlive the app.
    pub fn cancel_all(&self) {
        let pids: Vec<u32> = self.0.lock().unwrap().keys().copied().collect();
        for pid in pids {
            self.cancel(pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_of_engine_lines_returns_events() {
        // Given / When / Then each §3.2 line type parses
        assert_eq!(
            serde_json::from_str::<EngineEvent>(r#"{"type":"progress","stage":"beats","pct":40}"#)
                .unwrap(),
            EngineEvent::Progress {
                stage: "beats".into(),
                pct: 40,
                elapsed_sec: None
            }
        );
        assert_eq!(
            serde_json::from_str::<EngineEvent>(r#"{"type":"result","path":"/a/song.json"}"#)
                .unwrap(),
            EngineEvent::Result {
                path: "/a/song.json".into()
            }
        );
        assert_eq!(
            serde_json::from_str::<EngineEvent>(r#"{"type":"error","message":"boom"}"#).unwrap(),
            EngineEvent::Error {
                message: "boom".into(),
                stderr: String::new()
            }
        );
    }

    #[test]
    fn transcribe_args_adds_the_mode_then_meter_and_allinone_only_when_asked() {
        // Given / When / Then
        let args = |meter, sections, mode: &str| {
            transcribe_args("/a.mp3".into(), "/a.json".into(), meter, sections, mode.into())
                .join(" ")
        };
        assert_eq!(
            args(None, false, "fast"),
            "transcribe /a.mp3 --out /a.json --mode fast"
        );
        assert_eq!(
            args(Some("6/8".into()), true, "accurate"),
            "transcribe /a.mp3 --out /a.json --mode accurate --meter 6/8 --beats allinone"
        );
    }

    #[test]
    fn from_str_of_record_progress_keeps_elapsed_seconds() {
        // Given / When / Then
        assert_eq!(
            serde_json::from_str::<EngineEvent>(
                r#"{"type":"progress","stage":"record","pct":0,"elapsedSec":3}"#
            )
            .unwrap(),
            EngineEvent::Progress {
                stage: "record".into(),
                pct: 0,
                elapsed_sec: Some(3)
            }
        );
    }

    fn packaged(root: &std::path::Path) -> Packaged {
        packaged_at(root, "1.2.3")
    }

    fn packaged_at(root: &std::path::Path, version: &str) -> Packaged {
        Packaged {
            uv: root.join("uv"),
            project: root.join("engine"),
            env: root.join("env"),
            version: version.into(),
        }
    }

    #[test]
    fn engine_command_prefers_the_override_then_the_packaged_engine_then_dev() {
        // Given a packaged engine
        let packaged = packaged(std::path::Path::new("/data"));

        // When / Then
        assert_eq!(
            engine_command(Some("python -m x".into()), Some(&packaged)),
            ["python", "-m", "x"]
        );
        assert_eq!(
            engine_command(Some(" ".into()), Some(&packaged)),
            [venv_engine(Path::new("/data/env")).display().to_string()]
        );
        let dev = engine_command(None, None);
        assert_eq!(dev[..2], ["uv", "run"]);
        assert!(dev[3].ends_with("/../../engine"));
    }

    #[test]
    fn missing_dev_engine_only_flags_the_dev_command_without_its_env() {
        // Given / When / Then
        assert_eq!(missing_dev_engine(&["python".into()]), None);
        let installed = venv_engine(&Path::new(DEV_PROJECT).join(".venv")).exists();
        assert_eq!(
            missing_dev_engine(&engine_command(None, None)).is_some(),
            !installed
        );
    }

    #[test]
    fn setup_command_installs_the_env_with_the_bundled_wheel_then_runs_setup() {
        // Given a fake uv that logs its calls and creates the engine, and a bundled madmom wheel
        let root = std::env::temp_dir().join(format!("madsister-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("engine/wheels")).unwrap();
        std::fs::write(root.join("engine/wheels/madmom-0.17-cp311.whl"), "").unwrap();
        // `run … python -c <script> …` runs the setup script with the system's python3.
        let fake_uv = r#"#!/bin/sh
if [ "$1" = run ]; then
  while [ "$1" != python ]; do shift; done
  shift
  exec python3 "$@"
fi
echo "$*" >> "$UV_PROJECT_ENVIRONMENT.log"
echo "Resolved 110 packages"
mkdir -p "$UV_PROJECT_ENVIRONMENT/bin"
cat > "$UV_PROJECT_ENVIRONMENT/bin/madsister-engine" <<'EOF'
#!/bin/sh
echo '{"type":"progress","stage":"setup","pct":50}'
echo '{"type":"result","path":"/models"}'
EOF
chmod +x "$UV_PROJECT_ENVIRONMENT/bin/madsister-engine"
"#;
        std::fs::write(root.join("uv"), fake_uv).unwrap();
        std::fs::set_permissions(
            root.join("uv"),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
        let packaged = packaged(&root);
        assert!(packaged.needs_setup());

        // When running the setup command to its end
        let (send, receive) = std::sync::mpsc::channel();
        Jobs::default()
            .spawn(&packaged.setup_command(&[]), false, move |e| {
                send.send(e).unwrap()
            })
            .unwrap();
        let events: Vec<EngineEvent> = receive.iter().collect();

        // Then progress goes from install to setup, ends with setup's result, and the env is stamped
        assert_eq!(
            events,
            [
                EngineEvent::Progress {
                    stage: "install".into(),
                    pct: 0,
                    elapsed_sec: None
                },
                EngineEvent::Progress {
                    stage: "setup".into(),
                    pct: 50,
                    elapsed_sec: None
                },
                EngineEvent::Result {
                    path: "/models".into()
                },
            ]
        );
        assert!(!packaged.needs_setup());
        // And uv synced the frozen lock without madmom, then installed the wheel into the env
        let calls = std::fs::read_to_string(root.join("env.log")).unwrap();
        let calls: Vec<&str> = calls.lines().collect();
        assert!(calls[0].starts_with("sync --frozen --no-dev --no-editable --project "));
        assert!(calls[0].ends_with("--group record --no-install-package madmom"));
        assert!(calls[1].starts_with("pip install --python "));
        assert!(calls[1].ends_with("/engine/wheels/madmom-0.17-cp311.whl"));
        assert!(!calls[0].contains("beats-allinone"));
        assert!(!packaged.has("sections"));

        // When installing section detection on demand, then Demucs (accurate mode)
        let run = |packaged: &Packaged, features: &[&str]| {
            let features: Vec<String> = features.iter().map(|f| f.to_string()).collect();
            let (send, receive) = std::sync::mpsc::channel();
            Jobs::default()
                .spawn(&packaged.setup_command(&features), false, move |e| {
                    send.send(e).unwrap()
                })
                .unwrap();
            receive.iter().count()
        };
        run(&packaged, &["sections"]);
        assert!(packaged.has("sections") && !packaged.has("separate"));
        run(&packaged, &["separate"]);

        // Then uv synced with all-in-one, then with all-in-one kept beside Demucs (a sync is exact), and the env remembers both
        let calls = std::fs::read_to_string(root.join("env.log")).unwrap();
        let calls: Vec<&str> = calls.lines().collect();
        assert!(calls[2].contains("--group beats-allinone") && !calls[2].contains("--group separate"));
        assert!(calls[4].contains("--group beats-allinone --group separate"));
        assert!(packaged.has("sections") && packaged.has("separate"));

        // When an update runs the plain setup, Then both are dropped
        let updated = packaged_at(&root, "1.2.4");
        run(&updated, &[]);
        assert!(!updated.has("sections") && !updated.has("separate"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn from_str_of_invalid_json_or_unknown_type_fails() {
        // Given / When / Then a non-JSON line and an unknown type are rejected
        assert!(serde_json::from_str::<EngineEvent>("Downloading weights...").is_err());
        assert!(serde_json::from_str::<EngineEvent>(r#"{"type":"debug","message":"x"}"#).is_err());
    }
}
