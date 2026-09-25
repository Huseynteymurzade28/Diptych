use crate::config::{GroupBy, IconTheme, ViewMode};
use crate::core::Theme;
use crate::ui::state::AppState;
use gtk4::prelude::*;
use gtk4::{Align, Box, DropDown, Label, Orientation, Scale, Separator, StringList, Switch};
use std::rc::Rc;

/// Builds the full settings panel as a Box widget.
/// Every change goes through `AppState::update_config`, which saves and refreshes.
pub fn build_settings_panel(state: &Rc<AppState>) -> Box {
    let config = &state.config;
    let panel = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .css_classes(vec!["settings-panel".to_string()])
        .build();

    // ── Title ──
    let title = Label::builder()
        .label("⚙  Settings")
        .css_classes(vec!["inspector-title".to_string()])
        .halign(Align::Start)
        .build();
    panel.append(&title);
    panel.append(
        &Separator::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(4)
            .margin_bottom(4)
            .build(),
    );

    // ═══════════════════════════════════
    //  APPEARANCE
    // ═══════════════════════════════════
    panel.append(&section_title("APPEARANCE"));

    // Theme selector
    {
        let row = setting_row("Theme");
        let theme_names = Theme::all_names();
        let string_list = StringList::new(&theme_names);
        let dropdown = DropDown::builder().model(&string_list).build();

        // Set current selection
        let current_theme = config.borrow().theme.clone();
        for (i, name) in theme_names.iter().enumerate() {
            if *name == current_theme {
                dropdown.set_selected(i as u32);
                break;
            }
        }

        let state_c = state.clone();
        dropdown.connect_selected_notify(move |dd| {
            if let Some(name) = Theme::all_names().get(dd.selected() as usize) {
                state_c.apply_theme(name);
                state_c.update_config(|cfg| cfg.theme = name.to_string());
            }
        });
        row.append(&dropdown);
        panel.append(&row);
    }

    // Icon size slider
    {
        let row = setting_row("Icon Size");
        let scale = Scale::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        scale.set_range(24.0, 96.0);
        scale.set_value(config.borrow().icon_size as f64);
        scale.set_increments(4.0, 16.0);

        let size_label = Label::builder()
            .label(&format!("{}px", config.borrow().icon_size))
            .css_classes(vec!["settings-label".to_string()])
            .width_chars(5)
            .build();

        // Only update in memory on every tick; the content view re-renders
        // when settings close, and config is saved on window close or when
        // another setting changes.
        let state_c = state.clone();
        let size_label_c = size_label.clone();
        scale.connect_value_changed(move |s| {
            let val = s.value() as i32;
            state_c.config.borrow_mut().icon_size = val;
            size_label_c.set_label(&format!("{}px", val));
        });

        row.append(&scale);
        row.append(&size_label);
        panel.append(&row);
    }

    // View mode toggle
    {
        let row = setting_row("View Mode");
        let modes = StringList::new(&["Grid", "List", "Graph", "Tree"]);
        let dropdown = DropDown::builder().model(&modes).build();
        dropdown.set_selected(match config.borrow().view_mode {
            ViewMode::Grid => 0,
            ViewMode::List => 1,
            ViewMode::Graph => 2,
            ViewMode::Tree => 3,
        });

        let state_c = state.clone();
        dropdown.connect_selected_notify(move |dd| {
            let value = match dd.selected() {
                0 => ViewMode::Grid,
                1 => ViewMode::List,
                2 => ViewMode::Graph,
                _ => ViewMode::Tree,
            };
            state_c.update_config(|cfg| cfg.view_mode = value);
        });
        row.append(&dropdown);
        panel.append(&row);
    }

    // Icon theme selector
    {
        let row = setting_row("Icon Theme");
        let theme_names = IconTheme::all_names();
        let string_list = StringList::new(&theme_names);
        let dropdown = DropDown::builder().model(&string_list).build();

        let current_icon_theme = config.borrow().icon_theme.display_name();
        for (i, name) in theme_names.iter().enumerate() {
            if *name == current_icon_theme {
                dropdown.set_selected(i as u32);
                break;
            }
        }

        let state_c = state.clone();
        dropdown.connect_selected_notify(move |dd| {
            if let Some(name) = IconTheme::all_names().get(dd.selected() as usize) {
                state_c.update_config(|cfg| cfg.icon_theme = IconTheme::from_name(name));
            }
        });
        row.append(&dropdown);
        panel.append(&row);
    }

    panel.append(
        &Separator::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(4)
            .margin_bottom(4)
            .build(),
    );

    // ═══════════════════════════════════
    //  GROUPING
    // ═══════════════════════════════════
    panel.append(&section_title("GROUPING"));
    {
        let row = setting_row("Group By");
        let groups = StringList::new(&["None", "Type", "Date", "Name"]);
        let dropdown = DropDown::builder().model(&groups).build();
        dropdown.set_selected(match config.borrow().grouping {
            GroupBy::None => 0,
            GroupBy::Type => 1,
            GroupBy::Date => 2,
            GroupBy::Name => 3,
        });

        let state_c = state.clone();
        dropdown.connect_selected_notify(move |dd| {
            let value = match dd.selected() {
                1 => GroupBy::Type,
                2 => GroupBy::Date,
                3 => GroupBy::Name,
                _ => GroupBy::None,
            };
            state_c.update_config(|cfg| cfg.grouping = value);
        });
        row.append(&dropdown);
        panel.append(&row);
    }

    panel.append(
        &Separator::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(4)
            .margin_bottom(4)
            .build(),
    );

    // ═══════════════════════════════════
    //  METADATA TOGGLES
    // ═══════════════════════════════════
    panel.append(&section_title("METADATA DISPLAY"));

    // Show file size
    {
        let row = setting_row("Show File Size");
        let switch = Switch::builder()
            .active(config.borrow().show_file_size)
            .valign(Align::Center)
            .build();
        let state_c = state.clone();
        switch.connect_active_notify(move |s| {
            let value = s.is_active();
            state_c.update_config(|cfg| cfg.show_file_size = value);
        });
        row.append(&switch);
        panel.append(&row);
    }

    // Show modified date
    {
        let row = setting_row("Show Modified Date");
        let switch = Switch::builder()
            .active(config.borrow().show_modified_date)
            .valign(Align::Center)
            .build();
        let state_c = state.clone();
        switch.connect_active_notify(move |s| {
            let value = s.is_active();
            state_c.update_config(|cfg| cfg.show_modified_date = value);
        });
        row.append(&switch);
        panel.append(&row);
    }

    // Show hidden files
    {
        let row = setting_row("Show Hidden Files");
        let switch = Switch::builder()
            .active(config.borrow().show_hidden)
            .valign(Align::Center)
            .build();
        let state_c = state.clone();
        switch.connect_active_notify(move |s| {
            let value = s.is_active();
            state_c.update_config(|cfg| cfg.show_hidden = value);
        });
        row.append(&switch);
        panel.append(&row);
    }

    panel
}

fn section_title(text: &str) -> Label {
    Label::builder()
        .label(text)
        .css_classes(vec!["settings-section-title".to_string()])
        .halign(Align::Start)
        .margin_top(4)
        .build()
}

fn setting_row(label_text: &str) -> Box {
    let row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(4)
        .margin_bottom(4)
        .build();

    let label = Label::builder()
        .label(label_text)
        .css_classes(vec!["settings-label".to_string()])
        .halign(Align::Start)
        .hexpand(true)
        .xalign(0.0)
        .build();

    row.append(&label);
    row
}
