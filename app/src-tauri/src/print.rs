//! `madsister print`: writes songs' PDFs from a hidden window, with the PDF writer the GUI exports with (pdf.rs).

use serde::Serialize;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "usage: madsister print [--style fr|intl|latin] [--font kalam|patrick-hand|petaluma-script|petaluma-text|serif]
                       [--bars-per-row 2|4|8] [--compact | --no-compact] [--out-dir DIR] FILE...
Writes FILE's chart to <name>.pdf next to it, or in DIR. Unset options use the app's settings.";

#[derive(Debug, PartialEq)]
pub struct Job {
    pub song: PathBuf,
    pub pdf: PathBuf,
}

/// What the frontend renders: song paths (for its log) and the settings given on the command line, by `Settings` key.
#[derive(Debug, PartialEq, Serialize)]
pub struct Plan {
    pub songs: Vec<String>,
    pub overrides: Map<String, Value>,
}

#[derive(Debug, PartialEq)]
pub struct Batch {
    pub jobs: Vec<Job>,
    pub overrides: Map<String, Value>,
}

impl Batch {
    pub fn plan(&self) -> Plan {
        Plan {
            songs: self
                .jobs
                .iter()
                .map(|j| j.song.display().to_string())
                .collect(),
            overrides: self.overrides.clone(),
        }
    }
}

/// Parses the arguments after `print`. Option values aren't checked here: the frontend checks them against its settings.
pub fn parse_args(args: &[String]) -> Result<Batch, String> {
    let mut overrides = Map::new();
    let mut out_dir = None;
    let mut songs = Vec::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--style" => _ = overrides.insert("style".into(), value()?.into()),
            "--font" => _ = overrides.insert("font".into(), value()?.into()),
            "--bars-per-row" => {
                let v = value()?;
                let n: u64 = v
                    .parse()
                    .map_err(|_| format!("--bars-per-row: not a number: {v}"))?;
                overrides.insert("barsPerRow".into(), n.into());
            }
            "--compact" => _ = overrides.insert("compactPrint".into(), true.into()),
            "--no-compact" => _ = overrides.insert("compactPrint".into(), false.into()),
            "--out-dir" => out_dir = Some(PathBuf::from(value()?)),
            a if a.starts_with("--") => return Err(format!("unknown option {a}")),
            _ => songs.push(PathBuf::from(arg)),
        }
    }
    if songs.is_empty() {
        return Err("no song to print".into());
    }
    let jobs: Vec<Job> = songs
        .into_iter()
        .map(|song| Job {
            pdf: pdf_path(&song, out_dir.as_deref()),
            song,
        })
        .collect();
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
    let stem = [".madsister.json", ".json"]
        .iter()
        .find_map(|ext| name.strip_suffix(ext))
        .unwrap_or(&name);
    out_dir
        .or(song.parent())
        .unwrap_or(Path::new(""))
        .join(format!("{stem}.pdf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn writes_the_pdf_next_to_the_song() {
        let batch = parse_args(&args("songs/a.madsister.json b.json c")).unwrap();
        let pdfs: Vec<_> = batch.jobs.iter().map(|j| j.pdf.clone()).collect();
        assert_eq!(
            pdfs,
            [
                PathBuf::from("songs/a.pdf"),
                PathBuf::from("b.pdf"),
                PathBuf::from("c.pdf")
            ]
        );
        assert!(batch.overrides.is_empty());
    }

    #[test]
    fn writes_the_pdf_in_the_out_dir() {
        let batch = parse_args(&args("songs/a.madsister.json --out-dir pdf")).unwrap();
        assert_eq!(batch.jobs[0].pdf, PathBuf::from("pdf/a.pdf"));
    }

    #[test]
    fn keeps_the_options_given_by_settings_key() {
        let batch = parse_args(&args(
            "--style intl --font kalam --bars-per-row 8 --compact a.json --no-compact",
        ))
        .unwrap();
        assert_eq!(
            Value::Object(batch.overrides),
            serde_json::json!({ "style": "intl", "font": "kalam", "barsPerRow": 8, "compactPrint": false })
        );
    }

    #[test]
    fn rejects_bad_arguments() {
        assert_eq!(parse_args(&args("")), Err("no song to print".into()));
        assert_eq!(
            parse_args(&args("--out-dir pdf a/x.madsister.json b/x.json")),
            Err(format!(
                "two songs would print to {}",
                Path::new("pdf").join("x.pdf").display()
            ))
        );
        assert_eq!(
            parse_args(&args("a.json --font")),
            Err("--font needs a value".into())
        );
        assert_eq!(
            parse_args(&args("--bars-per-row x a.json")),
            Err("--bars-per-row: not a number: x".into())
        );
        assert_eq!(
            parse_args(&args("--color a.json")),
            Err("unknown option --color".into())
        );
    }
}
