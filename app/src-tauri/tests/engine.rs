use madsister_lib::engine::{EngineEvent, Jobs};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

fn spawn_fake(jobs: &Jobs, scenario: &str) -> (u32, Receiver<EngineEvent>) {
    let (tx, rx) = mpsc::channel();
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake-engine.sh");
    let argv = ["sh", script, scenario].map(String::from);
    let job_id = jobs
        .spawn(&argv, move |event| {
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
                pct: 0
            },
            EngineEvent::Progress {
                stage: "chords".into(),
                pct: 50
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
    let (job_id, rx) = spawn_fake(&jobs, "hang");
    let child_pid = match rx.recv().unwrap() {
        EngineEvent::Progress { stage, .. } => stage,
        other => panic!("expected progress, got {other:?}"),
    };

    // When cancelling the job
    jobs.cancel(job_id);
    let event = rx.recv_timeout(Duration::from_secs(5));

    // Then the job ends as cancelled at once (the child held stdout open), and the child is gone (reaped soon after)
    assert_eq!(event, Ok(EngineEvent::Cancelled));
    let is_child_alive = || {
        Command::new("kill")
            .args(["-0", &child_pid])
            .status()
            .unwrap()
            .success()
    };
    let is_child_gone = (0..50).any(|_| {
        thread::sleep(Duration::from_millis(100));
        !is_child_alive()
    });
    assert!(is_child_gone, "child {child_pid} still alive");
}
