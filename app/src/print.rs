//! `madsister print`: writes songs' PDFs with the PDF writer the GUI exports with (pdf.rs), without opening a window.

use std::path::{Path, PathBuf};

use crate::export::sheet::to_sheet;
use crate::model::display::DisplayStyle;
use crate::model::song::parse_song;
use crate::pdf;
use crate::settings::{BARS_PER_ROW, ChartFont, Settings, style_from_id};

pub const USAGE: &str = "usage: madsister print [--style fr|intl|latin] [--font kalam|patrick-hand|petaluma-script|petaluma-text|serif]
                       [--bars-per-row 2|4|8] [--compact | --no-compact] [--out-dir DIR] FILE...
Writes FILE's chart to <name>.pdf next to it, or in DIR. Unset options use the app's settings.";

#[derive(Debug, PartialEq)]
pub struct Job {
    pub song: PathBuf,
    pub pdf: PathBuf,
}

/// The settings given on the command line, over the saved ones.
#[derive(Debug, Default, PartialEq)]
pub struct Overrides {
    pub style: Option<DisplayStyle>,
    pub font: Option<ChartFont>,
    pub bars_per_row: Option<usize>,
    pub compact_print: Option<bool>,
}

impl Overrides {
    pub fn apply(&self, settings: Settings) -> Settings {
        Settings {
            style: self.style.unwrap_or(settings.style),
            font: self.font.unwrap_or(settings.font),
            bars_per_row: self.bars_per_row.unwrap_or(settings.bars_per_row),
            compact_print: self.compact_print.unwrap_or(settings.compact_print),
            ..settings
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Batch {
    pub jobs: Vec<Job>,
    pub overrides: Overrides,
}

fn invalid(option: &str, value: &str) -> String {
    format!("invalid {option}: {value:?}")
}

/// Parses the arguments after `print`.
pub fn parse_args(args: &[String]) -> Result<Batch, String> {
    let mut overrides = Overrides::default();
    let mut out_dir = None;
    let mut songs = Vec::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--style" => {
                let v = value()?;
                overrides.style = Some(style_from_id(&v).ok_or_else(|| invalid(arg, &v))?);
            }
            "--font" => {
                let v = value()?;
                overrides.font = Some(ChartFont::from_id(&v).ok_or_else(|| invalid(arg, &v))?);
            }
            "--bars-per-row" => {
                let v = value()?;
                let n: usize = v.parse().map_err(|_| format!("--bars-per-row: not a number: {v}"))?;
                if !BARS_PER_ROW.contains(&n) {
                    return Err(invalid(arg, &v));
                }
                overrides.bars_per_row = Some(n);
            }
            "--compact" => overrides.compact_print = Some(true),
            "--no-compact" => overrides.compact_print = Some(false),
            "--out-dir" => out_dir = Some(PathBuf::from(value()?)),
            a if a.starts_with("--") => return Err(format!("unknown option {a}")),
            _ => songs.push(PathBuf::from(arg)),
        }
    }
    if songs.is_empty() {
        return Err("no song to print".into());
    }
    let jobs: Vec<Job> = songs.into_iter().map(|song| Job { pdf: pdf_path(&song, out_dir.as_deref()), song }).collect();
    // Same name in two folders, with --out-dir: the second PDF would replace the first.
    let mut pdfs = std::collections::HashSet::new();
    if let Some(job) = jobs.iter().find(|j| !pdfs.insert(&j.pdf)) {
        return Err(format!("two songs would print to {}", job.pdf.display()));
    }
    Ok(Batch { jobs, overrides })
}

/// `dir/song.madsister.json` → `<out_dir or dir>/song.pdf`.
fn pdf_path(song: &Path, out_dir: Option<&Path>) -> PathBuf {
    let name = song.file_name().unwrap_or_default().to_string_lossy();
    let stem = [".madsister.json", ".json"].iter().find_map(|ext| name.strip_suffix(ext)).unwrap_or(&name);
    out_dir.or(song.parent()).unwrap_or(Path::new("")).join(format!("{stem}.pdf"))
}

/// Writes the job's PDF; returns its absolute path.
fn print(job: &Job, settings: &Settings) -> Result<PathBuf, String> {
    let text = std::fs::read_to_string(&job.song).map_err(|e| format!("can't read it: {e}"))?;
    let bytes = pdf::render(&to_sheet(&parse_song(&text)?, settings))?;
    let pdf = std::path::absolute(&job.pdf).map_err(|e| e.to_string())?;
    if let Some(dir) = pdf.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    }
    std::fs::write(&pdf, bytes).map_err(|e| format!("can't write it: {e}"))?;
    Ok(pdf)
}

/// Runs `madsister print <args>` with the saved settings under the options; returns the exit code (1 if a song failed,
/// 2 on bad arguments).
pub fn run(args: &[String], saved: Settings) -> i32 {
    let batch = match parse_args(args) {
        Ok(batch) => batch,
        Err(e) => {
            eprintln!("madsister print: {e}\n{USAGE}");
            return 2;
        }
    };
    let settings = batch.overrides.apply(saved);
    let mut has_failed = false;
    for job in &batch.jobs {
        match print(job, &settings) {
            Ok(pdf) => eprintln!("{} → {}", job.song.display(), pdf.display()),
            Err(e) => {
                has_failed = true;
                eprintln!("{}: {e}", job.song.display());
            }
        }
    }
    i32::from(has_failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn writes_the_pdf_next_to_the_song() {
        let batch = parse_args(&args("songs/a.madsister.json b.json c")).unwrap();
        let pdfs: Vec<_> = batch.jobs.iter().map(|j| j.pdf.clone()).collect();
        assert_eq!(pdfs, [PathBuf::from("songs/a.pdf"), PathBuf::from("b.pdf"), PathBuf::from("c.pdf")]);
        assert_eq!(batch.overrides, Overrides::default());
    }

    #[test]
    fn writes_the_pdf_in_the_out_dir() {
        let batch = parse_args(&args("songs/a.madsister.json --out-dir pdf")).unwrap();
        assert_eq!(batch.jobs[0].pdf, PathBuf::from("pdf/a.pdf"));
    }

    #[test]
    fn keeps_the_options_given_over_the_saved_settings() {
        // Given
        let batch = parse_args(&args("--style intl --font kalam --bars-per-row 8 --compact a.json --no-compact")).unwrap();
        let saved = Settings { compact_print: true, ..Settings::parse(&Value::Null) };

        // When
        let settings = batch.overrides.apply(saved.clone());

        // Then
        assert_eq!(
            settings,
            Settings {
                style: DisplayStyle::Intl,
                font: ChartFont::Kalam,
                bars_per_row: 8,
                compact_print: false,
                ..saved
            }
        );
    }

    #[test]
    fn rejects_bad_arguments() {
        assert_eq!(parse_args(&args("")), Err("no song to print".into()));
        assert_eq!(
            parse_args(&args("--out-dir pdf a/x.madsister.json b/x.json")),
            Err(format!("two songs would print to {}", Path::new("pdf").join("x.pdf").display()))
        );
        assert_eq!(parse_args(&args("a.json --font")), Err("--font needs a value".into()));
        assert_eq!(parse_args(&args("--bars-per-row x a.json")), Err("--bars-per-row: not a number: x".into()));
        assert_eq!(parse_args(&args("--bars-per-row 3 a.json")), Err("invalid --bars-per-row: \"3\"".into()));
        assert_eq!(parse_args(&args("--font comic a.json")), Err("invalid --font: \"comic\"".into()));
        assert_eq!(parse_args(&args("--style de a.json")), Err("invalid --style: \"de\"".into()));
        assert_eq!(parse_args(&args("--color a.json")), Err("unknown option --color".into()));
    }

    #[test]
    fn run_writes_each_song_pdf_and_fails_for_a_bad_one() {
        // Given a valid song and a file that isn't one
        let dir = std::env::temp_dir().join(format!("madsister-print-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let song = dir.join("good.madsister.json");
        std::fs::write(
            &song,
            r#"{ "version": 1, "meta": { "title": "Good", "meter": { "beats": 4, "unit": 4 } }, "sections": [
                { "id": "a", "label": "A", "bars": [{ "chords": [{ "chord": "C:maj", "beats": 4 }] }] }
            ] }"#,
        )
        .unwrap();
        let bad = dir.join("bad.json");
        std::fs::write(&bad, "{}").unwrap();
        let out = dir.join("pdf");

        // When
        let code = run(
            &[song.display().to_string(), bad.display().to_string(), "--out-dir".into(), out.display().to_string()],
            Settings::parse(&Value::Null),
        );

        // Then the good song is printed, and the exit code says one failed
        assert_eq!(code, 1);
        assert!(std::fs::read(out.join("good.pdf")).unwrap().starts_with(b"%PDF"));
        assert!(!out.join("bad.pdf").exists());
        assert_eq!(run(&[], Settings::parse(&Value::Null)), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
