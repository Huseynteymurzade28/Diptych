use crate::config::keybindings::BINDABLE;
use crate::ui::state::AppState;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Command palette (Ctrl+Shift+P / Ctrl+K)
// ═══════════════════════════════════════════════
//
// Every `win.*` action from keybindings.toml plus the custom actions that
// apply to the selection, filtered as you type. Runs the exact same
// actions as the menus and shortcuts.

struct Command {
    title: String,
    /// Shown dimmed after the title ("Custom action").
    note: Option<&'static str>,
    /// `win.name` plus its target, if any.
    action: String,
    target: Option<glib::Variant>,
    /// Detailed name for looking up the shortcut ("win.view-mode::grid").
    detailed: String,
}

fn commands(state: &Rc<AppState>) -> Vec<Command> {
    let mut list = vec![];
    for b in BINDABLE {
        if b.name == "command-palette" {
            continue;
        }
        let (name, target) = match b.name.split_once("::") {
            Some((name, target)) => (name, Some(target.to_variant())),
            None => (b.name, None),
        };
        let enabled = state
            .window
            .lookup_action(name)
            .is_some_and(|a| a.is_enabled());
        if !enabled {
            continue;
        }
        list.push(Command {
            title: b.title.to_string(),
            note: None,
            action: format!("win.{}", name),
            target,
            detailed: format!("win.{}", b.name),
        });
    }
    for (index, action) in state.applicable_actions() {
        list.push(Command {
            title: action.name,
            note: Some("Custom action"),
            action: "win.run-action".into(),
            target: Some((index as i32).to_variant()),
            detailed: format!("win.run-action({})", index),
        });
    }
    list
}

pub fn present(state: &Rc<AppState>) {
    // Only on the bare window: the palette over another dialog would run
    // commands behind it.
    if state.window.visible_dialog().is_some() {
        return;
    }
    let commands = Rc::new(commands(state));
    let accels = |detailed: &str| -> String {
        state
            .window
            .application()
            .map(|app| app.accels_for_action(detailed).join(" "))
            .unwrap_or_default()
    };

    let entry = gtk4::SearchEntry::builder()
        .placeholder_text("Type a command…")
        .hexpand(true)
        .build();
    let list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Browse)
        .css_classes(["navigation-sidebar"])
        .build();
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    let empty = adw::StatusPage::builder()
        .icon_name("edit-find-symbolic")
        .title("No Matching Commands")
        .vexpand(true)
        .visible(false)
        .css_classes(["compact"])
        .build();

    let body = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();
    body.append(&scroll);
    body.append(&empty);

    let header = adw::HeaderBar::builder()
        .title_widget(&entry)
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&body));

    let dialog = adw::Dialog::builder()
        .title("Commands")
        .content_width(520)
        .content_height(440)
        .child(&toolbar)
        .build();

    // Rows shown for the current query, best match first: indices into `commands`.
    let shown: Rc<RefCell<Vec<usize>>> = Rc::default();
    let rows: Vec<gtk4::ListBoxRow> = commands
        .iter()
        .map(|c| command_row(c, &accels(&c.detailed)))
        .collect();

    let fill = {
        let (commands, list, shown, scroll, empty) = (
            commands.clone(),
            list.clone(),
            shown.clone(),
            scroll.clone(),
            empty.clone(),
        );
        move |query: &str| {
            while let Some(row) = list.row_at_index(0) {
                list.remove(&row);
            }
            let mut ranked: Vec<(i32, usize)> = commands
                .iter()
                .enumerate()
                .filter_map(|(i, c)| score(query, &c.title).map(|s| (s, i)))
                .collect();
            // Stable: equal scores keep the menu order.
            ranked.sort_by_key(|(s, _)| -s);
            *shown.borrow_mut() = ranked.iter().map(|(_, i)| *i).collect();
            for (_, i) in &ranked {
                list.append(&rows[*i]);
            }
            list.select_row(list.row_at_index(0).as_ref());
            scroll.set_visible(!ranked.is_empty());
            empty.set_visible(ranked.is_empty());
        }
    };
    fill("");
    entry.connect_search_changed(move |e| fill(&e.text()));

    let run = {
        // Weak: these closures live inside the dialog.
        let (dialog, state, commands, shown) = (
            dialog.downgrade(),
            state.clone(),
            commands.clone(),
            shown.clone(),
        );
        move |position: i32| {
            let Some(&i) = shown.borrow().get(position.max(0) as usize) else {
                return;
            };
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
            let (state, commands) = (state.clone(), commands.clone());
            // After the dialog is gone, so dialogs the command opens
            // (rename, delete, Customize) aren't stacked on top of it.
            glib::idle_add_local_once(move || {
                let c = &commands[i];
                if let Err(e) =
                    WidgetExt::activate_action(&state.window, &c.action, c.target.as_ref())
                {
                    state.toast(&format!("Couldn’t run “{}”: {}", c.title, e));
                }
            });
        }
    };
    {
        let run = run.clone();
        list.connect_row_activated(move |_, row| run(row.index()));
    }
    {
        let list = list.clone();
        entry.connect_activate(move |_| {
            if let Some(row) = list.selected_row() {
                run(row.index());
            }
        });
    }
    {
        let dialog = dialog.downgrade();
        entry.connect_stop_search(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
        });
    }
    // Up/Down move through the list while typing.
    {
        let (list, scroll) = (list.clone(), scroll.clone());
        let keys = gtk4::EventControllerKey::new();
        keys.connect_key_pressed(move |_, key, _, _| {
            let step = match key {
                gtk4::gdk::Key::Down => 1,
                gtk4::gdk::Key::Up => -1,
                _ => return glib::Propagation::Proceed,
            };
            let current = list.selected_row().map(|r| r.index()).unwrap_or(0);
            if let Some(row) = list.row_at_index((current + step).max(0)) {
                list.select_row(Some(&row));
                if let Some(bounds) = row.compute_bounds(&list) {
                    let y = bounds.y() as f64;
                    scroll
                        .vadjustment()
                        .clamp_page(y, y + bounds.height() as f64);
                }
            }
            glib::Propagation::Stop
        });
        entry.add_controller(keys);
    }

    dialog.present(Some(&state.window));
    entry.grab_focus();
}

fn command_row(c: &Command, accels: &str) -> gtk4::ListBoxRow {
    let line = gtk4::Box::builder()
        .spacing(12)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();
    line.append(
        &gtk4::Label::builder()
            .label(&c.title)
            .xalign(0.0)
            .ellipsize(pango::EllipsizeMode::End)
            .build(),
    );
    if let Some(note) = c.note {
        line.append(
            &gtk4::Label::builder()
                .label(note)
                .css_classes(["dim-label", "caption"])
                .build(),
        );
    }
    let spacer = gtk4::Box::builder().hexpand(true).build();
    line.append(&spacer);
    if !accels.is_empty() {
        line.append(&gtk4::ShortcutLabel::new(accels));
    }
    gtk4::ListBoxRow::builder().child(&line).build()
}

/// How well `query` matches `text`: `None` unless every query character
/// appears in order (case-insensitive). Higher is better; a prefix beats
/// word starts, which beat scattered letters.
pub fn score(query: &str, text: &str) -> Option<i32> {
    let query: Vec<char> = query
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| !c.is_whitespace())
        .collect();
    if query.is_empty() {
        return Some(0);
    }
    let text: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    let mut score = 0;
    let mut qi = 0;
    let mut previous: Option<usize> = None;
    for (ti, &c) in text.iter().enumerate() {
        if qi == query.len() {
            break;
        }
        if c != query[qi] {
            continue;
        }
        let word_start = ti == 0 || !text[ti - 1].is_alphanumeric();
        score += 1;
        if word_start {
            score += 8;
        }
        if previous == Some(ti.wrapping_sub(1)) {
            score += 5;
        }
        if ti == qi {
            // Still matching the very beginning.
            score += 10;
        }
        previous = Some(ti);
        qi += 1;
    }
    (qi == query.len()).then_some(score - text.len() as i32 / 8)
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn matches_in_order_only() {
        assert!(score("hid", "Show hidden files").is_some());
        assert!(score("shf", "Show hidden files").is_some());
        assert!(score("fsh", "Show hidden files").is_none());
        assert!(score("", "Anything").is_some());
        assert!(score("  ", "Anything").is_some());
        assert!(score("ç", "Açık").is_some());
    }

    #[test]
    fn ranks_prefixes_and_word_starts_first() {
        let rank = |q: &str, items: &[&str]| {
            let mut v: Vec<(i32, String)> = items
                .iter()
                .filter_map(|t| score(q, t).map(|s| (s, t.to_string())))
                .collect();
            v.sort_by_key(|(s, _)| -s);
            v.into_iter().map(|(_, t)| t).collect::<Vec<_>>()
        };
        assert_eq!(
            rank("re", &["Move to trash", "Rename", "Refresh"])[..2],
            ["Rename", "Refresh"]
        );
        assert_eq!(rank("gv", &["Next view mode", "Grid view"])[0], "Grid view");
        assert_eq!(
            rank("del", &["Clear selection", "Delete permanently"])[0],
            "Delete permanently"
        );
    }
}
