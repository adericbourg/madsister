// Engine bridge (spec §3.2, F-IN-4): spawns `madsister-engine`, forwards its JSON Lines, kills it on cancel.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt; // ponytail: unix only (Linux, macOS), Windows is out of scope (D4)
use std::path::PathBuf;
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

pub fn parse_line(line: &str) -> serde_json::Result<EngineEvent> {
    serde_json::from_str(line)
}

/// The engine installed by `setup-engine.sh` in a packaged app (`tauri build`): `uv` is a sidecar next to the app's
/// binary, the `engine/` project a resource, the env lives in the app data dir.
pub struct Packaged {
    pub uv: PathBuf,
    pub project: PathBuf,
    pub env: PathBuf,
    pub version: String,
}

const SETUP_SCRIPT: &str = include_str!("../setup-engine.sh");

impl Packaged {
    /// No env yet, or one set up by another app version.
    pub fn needs_setup(&self) -> bool {
        std::fs::read_to_string(self.env.join(".madsister-version")).ok()
            != Some(self.version.clone())
    }

    /// All-in-one (section detection) is installed for this app version: `setup_command(true)` ran after the last setup.
    pub fn has_sections(&self) -> bool {
        std::fs::read_to_string(self.env.join(".madsister-sections")).ok()
            == Some(self.version.clone())
    }

    /// `sections`: also install all-in-one (slow opt-in, on demand).
    pub fn setup_command(&self, sections: bool) -> Vec<String> {
        let path = |p: &PathBuf| p.display().to_string();
        let mut command = vec![
            "sh".into(),
            "-c".into(),
            SETUP_SCRIPT.into(),
            "sh".into(),
            path(&self.uv),
            path(&self.project),
            path(&self.env),
            self.version.clone(),
        ];
        if sections {
            command.push("sections".into());
        }
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
        (_, Some(packaged)) => vec![packaged
            .env
            .join("bin/madsister-engine")
            .display()
            .to_string()],
        _ => {
            let project = concat!(env!("CARGO_MANIFEST_DIR"), "/../../engine");
            [
                "uv",
                "run",
                "--project",
                project,
                "--no-sync",
                "madsister-engine",
            ]
            .map(String::from)
            .to_vec()
        }
    }
}

/// `transcribe` arguments after the engine command. `sections`: all-in-one beats + sections (slow opt-in, ~18× madmom).
pub fn transcribe_args(
    audio_path: String,
    out_path: String,
    meter: Option<String>,
    sections: bool,
) -> Vec<String> {
    let mut args = vec!["transcribe".into(), audio_path, "--out".into(), out_path];
    if let Some(meter) = meter {
        args.extend(["--meter".into(), meter]);
    }
    if sections {
        args.extend(["--beats".into(), "allinone".into()]);
    }
    args
}

pub fn fetch_args(url: String, out_dir: String) -> Vec<String> {
    vec!["fetch".into(), url, "--out-dir".into(), out_dir]
}

pub fn record_args(out_path: String) -> Vec<String> {
    vec!["record".into(), "--out".into(), out_path]
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
                match parse_line(&line) {
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
    fn parse_line_of_engine_lines_returns_events() {
        // Given / When / Then each §3.2 line type parses
        assert_eq!(
            parse_line(r#"{"type":"progress","stage":"beats","pct":40}"#).unwrap(),
            EngineEvent::Progress {
                stage: "beats".into(),
                pct: 40,
                elapsed_sec: None
            }
        );
        assert_eq!(
            parse_line(r#"{"type":"result","path":"/a/song.json"}"#).unwrap(),
            EngineEvent::Result {
                path: "/a/song.json".into()
            }
        );
        assert_eq!(
            parse_line(r#"{"type":"error","message":"boom"}"#).unwrap(),
            EngineEvent::Error {
                message: "boom".into(),
                stderr: String::new()
            }
        );
    }

    #[test]
    fn transcribe_args_adds_meter_and_allinone_only_when_asked() {
        // Given / When / Then
        let args = |meter, sections| {
            transcribe_args("/a.mp3".into(), "/a.json".into(), meter, sections).join(" ")
        };
        assert_eq!(args(None, false), "transcribe /a.mp3 --out /a.json");
        assert_eq!(
            args(Some("6/8".into()), true),
            "transcribe /a.mp3 --out /a.json --meter 6/8 --beats allinone"
        );
    }

    #[test]
    fn parse_line_of_record_progress_keeps_elapsed_seconds() {
        // Given / When / Then
        assert_eq!(
            parse_line(r#"{"type":"progress","stage":"record","pct":0,"elapsedSec":3}"#).unwrap(),
            EngineEvent::Progress {
                stage: "record".into(),
                pct: 0,
                elapsed_sec: Some(3)
            }
        );
    }

    #[test]
    fn fetch_args_and_record_args_build_the_engine_verbs() {
        // Given / When / Then
        assert_eq!(
            fetch_args("https://x/v".into(), "/data/sources".into()).join(" "),
            "fetch https://x/v --out-dir /data/sources"
        );
        assert_eq!(
            record_args("/data/sources/r.wav".into()).join(" "),
            "record --out /data/sources/r.wav"
        );
    }

    fn packaged(root: &std::path::Path) -> Packaged {
        Packaged {
            uv: root.join("uv"),
            project: root.join("engine"),
            env: root.join("env"),
            version: "1.2.3".into(),
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
            ["/data/env/bin/madsister-engine"]
        );
        let dev = engine_command(None, None);
        assert_eq!(dev[..2], ["uv", "run"]);
        assert!(dev[3].ends_with("/../../engine"));
    }

    #[test]
    fn setup_command_installs_the_env_with_the_bundled_wheel_then_runs_setup() {
        // Given a fake uv that logs its calls and creates the engine, and a bundled madmom wheel
        let root = std::env::temp_dir().join(format!("madsister-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("engine/wheels")).unwrap();
        std::fs::write(root.join("engine/wheels/madmom-0.17-cp311.whl"), "").unwrap();
        let fake_uv = r#"#!/bin/sh
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
            .spawn(&packaged.setup_command(false), false, move |e| {
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
        assert!(!packaged.has_sections());

        // When installing section detection on demand
        let run = |sections| {
            let (send, receive) = std::sync::mpsc::channel();
            Jobs::default()
                .spawn(&packaged.setup_command(sections), false, move |e| {
                    send.send(e).unwrap()
                })
                .unwrap();
            receive.iter().count()
        };
        run(true);

        // Then uv synced with all-in-one and the env remembers it, until a plain setup (an update) drops it
        let calls = std::fs::read_to_string(root.join("env.log")).unwrap();
        assert!(calls.lines().nth(2).unwrap().contains("--group beats-allinone"));
        assert!(packaged.has_sections());
        run(false);
        assert!(!packaged.has_sections());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parse_line_of_invalid_json_or_unknown_type_fails() {
        // Given / When / Then a non-JSON line and an unknown type are rejected
        assert!(parse_line("Downloading weights...").is_err());
        assert!(parse_line(r#"{"type":"debug","message":"x"}"#).is_err());
    }
}
