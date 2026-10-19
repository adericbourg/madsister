use std::path::Path;
use std::rc::Rc;

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{Model, SharedString, VecModel};

use madsister::model::song::parse_song;
use madsister::settings::Settings;

mod view;

slint::include_modules!();

/// Shows the song file at `path` on the sheet; false when it can't be opened.
fn open(app: &AppWindow, settings: &Settings, path: &Path) -> bool {
    match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|text| parse_song(&text)) {
        Ok(song) => {
            app.set_chart(view::chart_view(&song, settings));
            app.set_has_song(true);
            true
        }
        // shortcut: errors go to stderr until the app shell (step 11) shows them.
        Err(e) => {
            eprintln!("can't open {}: {e}", path.display());
            false
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(("print", rest)) = args.split_first().map(|(command, rest)| (command.as_str(), rest)) {
        std::process::exit(madsister::print::run(rest, Settings::load()));
    }

    // The winit backend gives access to OS file drops (F-IN-1), which Slint's own drag & drop doesn't receive yet.
    let selector = slint::BackendSelector::new().backend_name("winit".into());
    // shortcut: winit has no Wayland drag & drop, so Linux runs under XWayland when it's there (no DISPLAY: native
    // Wayland without drops); go native Wayland once winit supports it.
    #[cfg(target_os = "linux")]
    let selector = if std::env::var_os("DISPLAY").is_some() {
        use slint::winit_030::winit::platform::x11::EventLoopBuilderExtX11;
        let mut builder = slint::winit_030::winit::event_loop::EventLoop::with_user_event();
        builder.with_x11();
        selector.with_winit_event_loop_builder(builder)
    } else {
        selector
    };
    selector.select()?;

    let settings = Rc::new(Settings::load());
    let app = AppWindow::new()?;
    let font = app.global::<ChartFont>();
    font.set_family(settings.font.family().into());
    font.set_has_bold(settings.font.has_bold());
    if let Some(path) = args.first() {
        open(&app, &settings, Path::new(path));
    }

    let recent = Rc::new(VecModel::<SharedString>::default());
    app.set_recent(recent.clone().into());
    let weak = app.as_weak();
    let dropped_into = recent.clone();
    let drop_settings = settings.clone();
    app.window().on_winit_window_event(move |_, event| {
        if let WindowEvent::DroppedFile(path) = event
            && let Some(app) = weak.upgrade()
            && open(&app, &drop_settings, path)
        {
            let path = SharedString::from(path.display().to_string());
            if let Some(i) = dropped_into.iter().position(|p| p == path) {
                dropped_into.remove(i);
            }
            dropped_into.insert(0, path);
        }
        EventResult::Propagate
    });

    let weak = app.as_weak();
    app.on_open_recent(move |path| {
        if let Some(app) = weak.upgrade() {
            open(&app, &settings, Path::new(path.as_str()));
        }
    });
    // Deferred: macOS native menus break if their structure changes while an item's activation is being handled.
    app.on_clear_recent(move || {
        let recent = recent.clone();
        slint::Timer::single_shot(std::time::Duration::ZERO, move || recent.set_vec(Vec::new()));
    });

    app.run()
}
