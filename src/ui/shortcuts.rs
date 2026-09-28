use crate::config::keybindings::{Scope, BINDABLE};
use crate::ui::state::AppState;
use adw::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Shortcuts: keybindings.toml + actions.toml keys → GTK
// ═══════════════════════════════════════════════
//
// Window-scope shortcuts become application accelerators. View-scope ones
// (Delete, F2, Escape, Ctrl+A) go to a controller on the file view, so text
// fields keep those keys. Custom actions bind `win.run-action(<index>)`.
//
// Runs again on every hot reload; everything set last time is replaced.

/// Applies the current keybindings and custom action keys.
/// Returns a message per shortcut that was skipped.
pub fn apply(state: &Rc<AppState>) -> Vec<String> {
    let keys = state.keybindings.borrow().clone();
    let actions = state.actions.borrow().clone();

    // (title, detailed action, scope, accelerators)
    let mut wanted: Vec<(String, String, Scope, Vec<String>)> = BINDABLE
        .iter()
        .map(|b| {
            (
                b.title.to_string(),
                format!("win.{}", b.name),
                b.scope,
                keys.get(b.name).to_vec(),
            )
        })
        .collect();
    for (i, a) in actions.0.iter().enumerate() {
        if let Some(key) = &a.key {
            wanted.push((
                format!("“{}”", a.name),
                format!("win.run-action({})", i),
                Scope::Window,
                vec![key.clone()],
            ));
        }
    }

    let mut errors = vec![];
    let mut taken: HashMap<String, String> = HashMap::new();
    let mut window_accels: Vec<(String, Vec<String>)> = vec![];
    let mut view_accels: Vec<(String, String)> = vec![];

    for (title, action, scope, accels) in wanted {
        let mut ok = vec![];
        for accel in accels {
            let Some((key, mods)) = gtk4::accelerator_parse(accel.as_str()) else {
                errors.push(format!("{}: “{}” isn’t a valid shortcut", title, accel));
                continue;
            };
            // "<Control>a" and "<Ctrl>A" are the same key.
            let canonical = gtk4::accelerator_name(key, mods).to_string();
            if let Some(owner) = taken.get(&canonical) {
                errors.push(format!(
                    "“{}” is used by both {} and {}; kept {}",
                    accel, owner, title, owner
                ));
                continue;
            }
            taken.insert(canonical.clone(), title.clone());
            ok.push(canonical);
        }
        match scope {
            Scope::Window => window_accels.push((action, ok)),
            Scope::View => view_accels.extend(ok.into_iter().map(|a| (a, action.clone()))),
        }
    }

    if let Some(app) = state.window.application() {
        // Drop accelerators of actions that no longer exist (custom
        // actions removed from actions.toml).
        for detailed in app.list_action_descriptions() {
            app.set_accels_for_action(&detailed, &[]);
        }
        for (action, accels) in &window_accels {
            let accels: Vec<&str> = accels.iter().map(String::as_str).collect();
            app.set_accels_for_action(action, &accels);
        }
    }
    state.file_view.set_shortcuts(&view_accels);

    for e in &errors {
        eprintln!("[keys] {}", e);
    }
    errors
}

/// The first shortcut of a `win.*` action, for menus and tooltips ("Ctrl+H").
pub fn label_for(window: &adw::ApplicationWindow, detailed: &str) -> Option<String> {
    let app = window.application()?;
    let accel = app.accels_for_action(detailed).into_iter().next()?;
    let (key, mods) = gtk4::accelerator_parse(accel.as_str())?;
    Some(gtk4::accelerator_get_label(key, mods).to_string())
}
