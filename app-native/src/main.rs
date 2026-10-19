use std::rc::Rc;

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{Model, SharedString, VecModel};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
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

    let app = AppWindow::new()?;
    let recent = Rc::new(VecModel::<SharedString>::default());
    app.set_recent(recent.clone().into());

    let weak = app.as_weak();
    let dropped_into = recent.clone();
    app.window().on_winit_window_event(move |_, event| {
        if let WindowEvent::DroppedFile(path) = event {
            let path = SharedString::from(path.display().to_string());
            eprintln!("dropped {path}");
            if let Some(app) = weak.upgrade() {
                app.set_dropped(path.clone());
            }
            if let Some(i) = dropped_into.iter().position(|p| p == path) {
                dropped_into.remove(i);
            }
            dropped_into.insert(0, path);
        }
        EventResult::Propagate
    });

    app.on_open_recent(|path| eprintln!("open recent {path}"));
    // Deferred: macOS native menus break if their structure changes while an item's activation is being handled.
    app.on_clear_recent(move || {
        let recent = recent.clone();
        slint::Timer::single_shot(std::time::Duration::ZERO, move || recent.set_vec(Vec::new()));
    });

    app.run()
}
