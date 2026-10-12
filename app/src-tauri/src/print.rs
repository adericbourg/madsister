//! `madsister print`: renders songs to PDF in a hidden window, with the webview the GUI prints with.

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

/// Prints the webview's page to `pdf`, as on paper (print media, A4), without a dialog. Blocks until it's written.
#[cfg(target_os = "linux")]
pub fn to_pdf(webview: &tauri::Webview, pdf: &Path) -> Result<(), String> {
    use webkit2gtk::PrintOperationExt;
    let uri = glib::filename_to_uri(pdf, None).map_err(|e| e.to_string())?;
    let (tx, rx) = std::sync::mpsc::channel();
    webview
        .with_webview(move |w| {
            let settings = gtk::PrintSettings::new();
            // GTK's file printer is named in the user's language.
            settings.set_printer(&glib::dgettext(Some("gtk30"), "Print to File"));
            settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI, Some(&uri));
            settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
            let setup = gtk::PageSetup::new();
            setup.set_paper_size(&gtk::PaperSize::new(Some(gtk::PAPER_NAME_A4)));
            let op = webkit2gtk::PrintOperation::new(&w.inner());
            op.set_print_settings(&settings);
            op.set_page_setup(&setup);
            // `failed` comes before `finished`: the receiver keeps the first.
            let failed = tx.clone();
            op.connect_failed(move |_, e| _ = failed.send(Err(e.to_string())));
            op.connect_finished(move |_| _ = tx.send(Ok(())));
            op.print();
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

#[cfg(target_os = "macos")]
pub fn to_pdf(webview: &tauri::Webview, pdf: &Path) -> Result<(), String> {
    use objc2::{sel, MainThreadMarker};
    use objc2_app_kit::{NSPrintInfo, NSPrintJobSavingURL, NSPrintSaveJob};
    use objc2_foundation::{NSSize, NSURL};
    use objc2_web_kit::WKWebView;

    let pdf = pdf.to_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    webview
        .with_webview(move |w| {
            let Some(mtm) = MainThreadMarker::new() else {
                _ = tx.send(Err("not on the main thread".into()));
                return;
            };
            // Safety: on macOS, the platform webview is a WKWebView, alive while the window is.
            let view = unsafe { &*w.inner().cast::<WKWebView>() };
            let Some(window) = view.window() else {
                _ = tx.send(Err("the webview has no window".into()));
                return;
            };
            // The app prints with no margins of its own, as the GUI does (wry's default): grid.css's @page sets them.
            let info = NSPrintInfo::sharedPrintInfo();
            info.setPaperSize(NSSize::new(595.28, 841.89)); // A4, in points
            info.setTopMargin(0.0);
            info.setRightMargin(0.0);
            info.setBottomMargin(0.0);
            info.setLeftMargin(0.0);
            let Some(url) = NSURL::from_file_path(&pdf) else {
                _ = tx.send(Err("not a file path".into()));
                return;
            };
            // Safety: the extern statics are AppKit's constants; the dictionary holds AnyObject values.
            unsafe {
                info.setJobDisposition(NSPrintSaveJob);
                info.dictionary().insert(NSPrintJobSavingURL, &*url);
            }
            let op = unsafe { view.printOperationWithPrintInfo(&info) };
            op.setShowsPrintPanel(false);
            op.setShowsProgressPanel(false);
            // WKWebView prints blank pages into a print view without a frame.
            if let Some(print_view) = op.view() {
                print_view.setFrame(view.frame());
            }
            let done = PrintDone::new(mtm, tx);
            // Modal, not runOperation: WKWebView draws its print pages asynchronously.
            unsafe {
                op.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                    &window,
                    Some(&done),
                    Some(sel!(printOperationDidRun:success:contextInfo:)),
                    std::ptr::null_mut(),
                );
            }
            // The operation doesn't retain its delegate; one small object per song, until the batch exits.
            std::mem::forget(done);
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

#[cfg(target_os = "macos")]
objc2::define_class!(
    /// The delegate `runOperationModalForWindow` calls once the PDF is written.
    #[unsafe(super(objc2::runtime::NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[ivars = std::sync::mpsc::Sender<Result<(), String>>]
    struct PrintDone;

    impl PrintDone {
        #[unsafe(method(printOperationDidRun:success:contextInfo:))]
        fn did_run(
            &self,
            _op: &objc2::runtime::AnyObject,
            success: objc2::runtime::Bool,
            _context: *mut std::ffi::c_void,
        ) {
            let result = success.as_bool().then_some(()).ok_or("the print failed".into());
            _ = objc2::DefinedClass::ivars(self).send(result);
        }
    }
);

#[cfg(target_os = "macos")]
impl PrintDone {
    fn new(
        mtm: objc2::MainThreadMarker,
        done: std::sync::mpsc::Sender<Result<(), String>>,
    ) -> objc2::rc::Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(done);
        unsafe { objc2::msg_send![super(this), init] }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn to_pdf(_webview: &tauri::Webview, _pdf: &Path) -> Result<(), String> {
    Err("printing to PDF from the command line isn't supported on this system yet".into())
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
            Err("two songs would print to pdf/x.pdf".into())
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
