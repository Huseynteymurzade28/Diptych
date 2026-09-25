use gtk4::prelude::*;
use gtk4::Window;
use std::time::Duration;

// ─── Window Snapshot (dev tool) ───
//
// `DIPTYCH_SNAPSHOT=/tmp/shot.png diptych` renders the window to a PNG
// shortly after it appears, then quits. Handy for README screenshots of
// themes and for checking layouts without a screenshot tool.
// `DIPTYCH_SNAPSHOT_DELAY_MS` (default 1500) sets how long to wait first,
// e.g. to edit layout.toml / theme.toml and capture the hot reload.

pub fn schedule_if_requested(window: &Window) {
    let Some(path) = std::env::var_os("DIPTYCH_SNAPSHOT") else {
        return;
    };
    let window = window.clone();
    // Give thumbnails and layout a moment to settle.
    let delay = std::env::var("DIPTYCH_SNAPSHOT_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1500);
    glib::timeout_add_local_once(Duration::from_millis(delay), move || {
        match render(&window) {
            Ok(()) => println!("[snapshot] Saved {}", path.to_string_lossy()),
            Err(e) => eprintln!("[snapshot] Failed: {}", e),
        }
        window.close();

        fn render(window: &Window) -> Result<(), String> {
            let path = std::env::var_os("DIPTYCH_SNAPSHOT").ok_or("no path")?;
            let (w, h) = (window.width(), window.height());
            let paintable = gtk4::WidgetPaintable::new(Some(window));
            let snapshot = gtk4::Snapshot::new();
            paintable.snapshot(&snapshot, w as f64, h as f64);
            let node = snapshot.to_node().ok_or("window rendered nothing")?;
            let renderer = window
                .native()
                .and_then(|n| n.renderer())
                .ok_or("window has no renderer")?;
            let texture = renderer.render_texture(&node, None);
            texture.save_to_png(&path).map_err(|e| e.to_string())
        }
    });
}
