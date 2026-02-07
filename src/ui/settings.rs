use crate::config::{AppConfig, GroupBy, IconTheme, ViewMode};
use crate::core::Theme;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, CssProvider, DropDown, Label, Orientation, Scale, Separator, StringList, Switch,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Builds the full settings panel as a Box widget.
/// Takes shared config + a "refresh" callback to apply changes live.
pub fn build_settings_panel(
    config: Rc<RefCell<AppConfig>>,
    css_provider: CssProvider,
    on_change: Rc<dyn Fn()>,
) -> Box {
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

        let config_c = config.clone();
        let css_c = css_provider.clone();
        let on_change_c = on_change.clone();
        dropdown.connect_selected_notify(move |dd| {
            let idx = dd.selected() as usize;
            let names = Theme::all_names();
            if let Some(name) = names.get(idx) {
                let theme = Theme::from_name(name);
                css_c.load_from_data(&theme.to_css());
                config_c.borrow_mut().theme = name.to_string();
                config_c.borrow().save();
                on_change_c();
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

        // Update label and apply live, but DON'T save to disk on every tick.
        // Config is persisted on window close or when another setting changes.
        let config_c = config.clone();
        let on_change_c = on_change.clone();
        let size_label_c = size_label.clone();
        scale.connect_value_changed(move |s| {
            let val = s.value() as i32;
            config_c.borrow_mut().icon_size = val;
            size_label_c.set_label(&format!("{}px", val));
            on_change_c();
        });

        row.append(&scale);
        row.append(&size_label);
        panel.append(&row);
    }

    // View mode toggle
    {
        let row = setting_row("View Mode");
        let modes = StringList::new(&["Grid", "List", "Graph"]);
        let dropdown = DropDown::builder().model(&modes).build();
        dropdown.set_selected(match config.borrow().view_mode {
            ViewMode::Grid => 0,
            ViewMode::List => 1,
            ViewMode::Graph => 2,
        });

        let config_c = config.clone();
        let on_change_c = on_change.clone();
        dropdown.connect_selected_notify(move |dd| {
            config_c.borrow_mut().view_mode = match dd.selected() {
                0 => ViewMode::Grid,
                1 => ViewMode::List,
                _ => ViewMode::Graph,
            };
            config_c.borrow().save();
            on_change_c();
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

        let config_c = config.clone();
        let on_change_c = on_change.clone();
        dropdown.connect_selected_notify(move |dd| {
            let idx = dd.selected() as usize;
            let names = IconTheme::all_names();
            if let Some(name) = names.get(idx) {
                config_c.borrow_mut().icon_theme = IconTheme::from_name(name);
                config_c.borrow().save();
                on_change_c();
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

        let config_c = config.clone();
        let on_change_c = on_change.clone();
        dropdown.connect_selected_notify(move |dd| {
            config_c.borrow_mut().grouping = match dd.selected() {
                1 => GroupBy::Type,
                2 => GroupBy::Date,
                3 => GroupBy::Name,
                _ => GroupBy::None,
            };
            config_c.borrow().save();
            on_change_c();
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
        let config_c = config.clone();
        let on_change_c = on_change.clone();
        switch.connect_active_notify(move |s| {
            config_c.borrow_mut().show_file_size = s.is_active();
            config_c.borrow().save();
            on_change_c();
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
        let config_c = config.clone();
        let on_change_c = on_change.clone();
        switch.connect_active_notify(move |s| {
            config_c.borrow_mut().show_modified_date = s.is_active();
            config_c.borrow().save();
            on_change_c();
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
        let config_c = config.clone();
        let on_change_c = on_change.clone();
        switch.connect_active_notify(move |s| {
            config_c.borrow_mut().show_hidden = s.is_active();
            config_c.borrow().save();
            on_change_c();
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
