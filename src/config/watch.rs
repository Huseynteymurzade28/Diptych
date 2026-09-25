use gio::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

// ─── Config File Watcher ───
//
// Calls back when files in the config directory change, so edits to
// theme.toml / user.css / layout.toml apply on save. Editors often write a
// file in several steps (truncate, write, rename), so events are coalesced.

const DEBOUNCE: Duration = Duration::from_millis(120);

/// Keeps the underlying monitors alive; drop it to stop watching.
pub struct Watch {
    _monitors: Vec<gio::FileMonitor>,
}

/// Watches `dirs` (non-recursively). `on_change` runs once per burst of
/// events for files whose name satisfies `relevant`.
pub fn watch(
    dirs: &[PathBuf],
    relevant: impl Fn(&Path) -> bool + 'static,
    on_change: impl Fn() + 'static,
) -> Watch {
    let relevant = Rc::new(relevant);
    let on_change: Rc<dyn Fn()> = Rc::new(on_change);
    let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let mut monitors = vec![];

    for dir in dirs {
        let _ = std::fs::create_dir_all(dir);
        let monitor = match gio::File::for_path(dir)
            .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[config] Cannot watch {}: {}", dir.display(), e);
                continue;
            }
        };
        let (relevant, on_change, pending) = (relevant.clone(), on_change.clone(), pending.clone());
        monitor.connect_changed(move |_, file, other, _| {
            // WATCH_MOVES reports "a.tmp renamed to a.toml": check both names.
            let hit = [Some(file), other]
                .into_iter()
                .flatten()
                .filter_map(|f| f.basename())
                .any(|name| relevant(&name));
            if !hit {
                return;
            }
            if let Some(id) = pending.borrow_mut().take() {
                id.remove();
            }
            let (on_change, pending_c) = (on_change.clone(), pending.clone());
            let id = glib::timeout_add_local_once(DEBOUNCE, move || {
                pending_c.borrow_mut().take();
                on_change();
            });
            *pending.borrow_mut() = Some(id);
        });
        monitors.push(monitor);
    }
    Watch {
        _monitors: monitors,
    }
}
