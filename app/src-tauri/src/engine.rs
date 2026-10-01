// Engine bridge (spec §3.2, F-IN-4): spawns `madsister-engine`, forwards its JSON Lines, kills it on cancel.
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::io::{self, BufRead, BufReader, Read};
use std::os::unix::process::CommandExt; // ponytail: unix only (Linux, macOS), Windows is out of scope (D4)
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// Engine lines, plus `cancelled` and the stderr tail on errors. Exactly one of result/error/cancelled ends a job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EngineEvent {
    Progress {
        stage: String,
        pct: u8,
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

/// `MADSISTER_ENGINE` (a command line, split on whitespace) or the dev engine. `--no-sync`: a plain `uv run` would
/// re-sync the env and remove the model dependency groups. Packaged resolution is M6.
pub fn engine_command() -> Vec<String> {
    match std::env::var("MADSISTER_ENGINE") {
        Ok(cmd) if !cmd.trim().is_empty() => cmd.split_whitespace().map(String::from).collect(),
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

/// Pids of the running jobs (a job id is its child's pid, which is also its process group id).
#[derive(Clone, Default)]
pub struct Jobs(Arc<Mutex<HashSet<u32>>>);

impl Jobs {
    pub fn spawn(
        &self,
        argv: &[String],
        emit: impl Fn(EngineEvent) + Send + 'static,
    ) -> io::Result<u32> {
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0) // its own group, so cancel also reaches uv's child and ffmpeg/Demucs
            .spawn()?;
        let pid = child.id();
        self.0.lock().unwrap().insert(pid);
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
            let was_cancelled = !jobs.0.lock().unwrap().remove(&pid);
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

    /// Kills the job's whole process group. No-op once the job has ended.
    pub fn cancel(&self, job_id: u32) {
        let mut live = self.0.lock().unwrap();
        if live.remove(&job_id) {
            // SAFETY: plain syscall; the group exists since its leader isn't reaped while it's in the set.
            unsafe { libc::killpg(job_id as libc::pid_t, libc::SIGKILL) };
        }
    }

    /// Called on app exit: the jobs are in their own process groups, so they'd outlive the app.
    pub fn cancel_all(&self) {
        let pids: Vec<u32> = self.0.lock().unwrap().iter().copied().collect();
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
                pct: 40
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
    fn parse_line_of_invalid_json_or_unknown_type_fails() {
        // Given / When / Then a non-JSON line and an unknown type are rejected
        assert!(parse_line("Downloading weights...").is_err());
        assert!(parse_line(r#"{"type":"debug","message":"x"}"#).is_err());
    }
}
