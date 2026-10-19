use madsister::engine::{EngineEvent, Jobs};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

fn spawn_fake(jobs: &Jobs, scenario: &str) -> (u32, Receiver<EngineEvent>) {
    spawn_fake_with_stdin(jobs, scenario, false)
}

fn spawn_fake_with_stdin(
    jobs: &Jobs,
    scenario: &str,
    has_stdin: bool,
) -> (u32, Receiver<EngineEvent>) {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake-engine.sh");
    spawn_argv(jobs, &["sh", script, scenario].map(String::from), has_stdin)
}

fn spawn_argv(jobs: &Jobs, argv: &[String], has_stdin: bool) -> (u32, Receiver<EngineEvent>) {
    let (tx, rx) = mpsc::channel();
    let job_id = jobs
        .spawn(argv, has_stdin, move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
    (job_id, rx)
}

fn error_of(events: &[EngineEvent]) -> (&str, &str) {
    match events {
        [EngineEvent::Error { message, stderr }] => (message, stderr),
        _ => panic!("expected a single error, got {events:?}"),
    }
}

#[test]
fn spawn_of_successful_engine_streams_progress_then_result() {
    // Given an engine that reports two stages then a result
    let jobs = Jobs::default();

    // When it runs to the end
    let (_, rx) = spawn_fake(&jobs, "success");
    let events: Vec<_> = rx.iter().collect();

    // Then every line is forwarded, the result last
    assert_eq!(
        events,
        [
            EngineEvent::Progress {
                stage: "decode".into(),
                pct: 0,
                elapsed_sec: None
            },
            EngineEvent::Progress {
                stage: "chords".into(),
                pct: 50,
                elapsed_sec: None
            },
            EngineEvent::Result {
                path: "/tmp/song.json".into()
            },
        ]
    );
}

#[test]
fn spawn_when_engine_fails_ends_with_its_error_and_output_tail() {
    // Given an engine that prints a stray stdout line, a traceback, then an error line and exits 1
    let jobs = Jobs::default();

    // When it runs
    let (_, rx) = spawn_fake(&jobs, "error");
    let events: Vec<_> = rx.iter().collect();

    // Then the error carries the engine message plus the stray line and the stderr tail
    let (message, stderr) = error_of(&events);
    assert_eq!(message, "boom");
    assert!(
        stderr.contains("not json") && stderr.contains("Traceback: boom"),
        "{stderr}"
    );

    // When the engine exits non-zero without an error line
    let (_, rx) = spawn_fake(&jobs, "crash");
    let events: Vec<_> = rx.iter().collect();

    // Then a synthetic error names the exit status
    assert!(error_of(&events).0.contains('3'), "{events:?}");
}

#[test]
fn cancel_kills_the_engine_and_its_children_then_ends_with_cancelled() {
    // Given an engine that hangs with a child process
    let jobs = Jobs::default();
    let (job_id, rx) = spawn_hang(&jobs);
    let child_pid = match rx.recv().unwrap() {
        EngineEvent::Progress { stage, .. } => stage,
        other => panic!("expected progress, got {other:?}"),
    };

    // When cancelling the job
    jobs.cancel(job_id);
    let event = rx.recv_timeout(Duration::from_secs(5));

    // Then the job ends as cancelled at once (the child held stdout open), and the child is gone (reaped soon after)
    assert_eq!(event, Ok(EngineEvent::Cancelled));
    let is_child_gone = (0..50).any(|_| {
        thread::sleep(Duration::from_millis(100));
        !is_alive(&child_pid)
    });
    assert!(is_child_gone, "child {child_pid} still alive");
}

#[cfg(unix)]
fn spawn_hang(jobs: &Jobs) -> (u32, Receiver<EngineEvent>) {
    spawn_fake(jobs, "hang")
}

/// Git Bash's `sh` children escape `taskkill /T` (its emulated fork loses the Windows parent link), so a native tree:
/// PowerShell starts `ping` as the hanging child, like python starting ffmpeg.
#[cfg(windows)]
fn spawn_hang(jobs: &Jobs) -> (u32, Receiver<EngineEvent>) {
    let script = r#"$p = Start-Process ping -ArgumentList '-n','60','127.0.0.1' -NoNewWindow -PassThru; Write-Output ('{"type":"progress","stage":"' + $p.Id + '","pct":0}'); Wait-Process -Id $p.Id"#;
    spawn_argv(jobs, &["powershell", "-NoProfile", "-Command", script].map(String::from), false)
}

#[cfg(unix)]
fn is_alive(pid: &str) -> bool {
    Command::new("kill").args(["-0", pid]).status().unwrap().success()
}

#[cfg(windows)]
fn is_alive(pid: &str) -> bool {
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).contains(pid)
}

#[test]
fn stop_writes_stop_to_the_engine_stdin_then_ends_with_its_result() {
    // Given a recording engine that waits for `stop` on its stdin
    let jobs = Jobs::default();
    let (job_id, rx) = spawn_fake_with_stdin(&jobs, "record", true);
    assert!(matches!(
        rx.recv().unwrap(),
        EngineEvent::Progress {
            elapsed_sec: Some(0),
            ..
        }
    ));

    // When stopping the job
    jobs.stop(job_id);
    let event = rx.recv_timeout(Duration::from_secs(5));

    // Then the engine finishes on its own and its result comes through
    assert_eq!(
        event,
        Ok(EngineEvent::Result {
            path: "/tmp/take.wav".into()
        })
    );
}
