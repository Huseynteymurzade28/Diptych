use crate::config::actions::{self, CustomAction, When};
use crate::config::keybindings::{self, Scope, BINDABLE};
use crate::config::layout::{self, Decorations, HeaderItem, HeaderLayout, InspectorField, Side};
use crate::config::{edit, GroupBy, IconTheme, OpenWith, ViewMode};
use crate::theme::Density;
use crate::ui::state::AppState;
use adw::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use toml_edit::DocumentMut;

// ═══════════════════════════════════════════════
//  Customize dialog
// ═══════════════════════════════════════════════
//
//   Appearance · Layout · Behavior · Shortcuts · Actions
//
// A front end for the same files people edit by hand: theme.toml,
// layout.toml, keybindings.toml, actions.toml (and config.toml for the
// few settings that live there). Every change is written straight back
// with comments intact and applied at once, so there's no Apply button,
// and a change that would make a file invalid is refused with a toast.

struct Ctx {
    state: Rc<AppState>,
    /// Weak: the dialog's own widgets hold the `Ctx`.
    dialog: glib::WeakRef<adw::PreferencesDialog>,
}

type C = Rc<Ctx>;

impl Ctx {
    fn pop_subpage(&self) {
        if let Some(dialog) = self.dialog.upgrade() {
            dialog.pop_subpage();
        }
    }

    /// Shows `result`'s error, if any, as a toast in the dialog.
    fn report(&self, result: Result<(), String>) -> bool {
        match result {
            Ok(()) => true,
            Err(e) => {
                match self.dialog.upgrade() {
                    Some(dialog) => {
                        eprintln!("[customize] {}", e);
                        dialog.add_toast(
                            adw::Toast::builder()
                                .title(glib::markup_escape_text(&e))
                                .timeout(6)
                                .build(),
                        );
                    }
                    None => {
                        self.state.toast(&e);
                    }
                }
                false
            }
        }
    }

    fn theme(&self, change: impl FnOnce(&mut DocumentMut) -> Result<(), String>) -> bool {
        self.report(self.state.theme.edit(change))
    }

    fn file(
        &self,
        file: &str,
        change: impl FnOnce(&mut DocumentMut) -> Result<(), String>,
    ) -> bool {
        self.report(self.state.edit_config_file(file, change))
    }
}

/// Opens the Customize dialog (`win.show-settings`).
pub fn present(state: &Rc<AppState>) {
    // Ctrl+, again while it's open: keep the one that's there.
    if let Some(open) = state.window.visible_dialog() {
        if open.is::<adw::PreferencesDialog>() {
            return;
        }
    }
    present_page(state, None);
}

fn present_page(state: &Rc<AppState>, page: Option<&str>) {
    let dialog = adw::PreferencesDialog::builder()
        .title("Customize")
        .search_enabled(true)
        .build();
    let ctx = Rc::new(Ctx {
        state: state.clone(),
        dialog: dialog.downgrade(),
    });
    dialog.add(&appearance_page(&ctx));
    dialog.add(&layout_page(&ctx));
    dialog.add(&behavior_page(&ctx));
    dialog.add(&shortcuts_page(&ctx));
    dialog.add(&actions_page(&ctx));
    if let Some(page) = page {
        dialog.set_visible_page_name(page);
    }
    dialog.present(Some(&state.window));
}

// ═══════════════════════════════════════════════
//  Appearance — theme.toml (+ icon settings in config.toml)
// ═══════════════════════════════════════════════

fn appearance_page(ctx: &C) -> adw::PreferencesPage {
    let page = page("Appearance", "applications-graphics-symbolic", "appearance");
    let theme = &ctx.state.theme;
    let file = theme.file();
    let current = theme.current().ok();
    let presets = Rc::new(theme.available());
    let names: Vec<&str> = presets.iter().map(|(_, name)| name.as_str()).collect();

    // ── Theme ──
    let error = theme.last_error().map(|e| {
        format!(
            "⚠ theme.toml has an error, so the previous theme is still in use: {}",
            e
        )
    });
    let group = new_group("Theme", error.as_deref());
    {
        let base = theme.base();
        let selected = presets.iter().position(|(id, _)| *id == base);
        let row = combo("Theme", &names, selected.unwrap_or(0));
        let (ctx, presets) = (ctx.clone(), presets.clone());
        row.connect_selected_notify(move |row| {
            if let Some((id, _)) = presets.get(row.selected() as usize) {
                ctx.theme(|d| edit::set(d, None, "base", id.as_str()));
            }
        });
        group.add(&row);
    }
    {
        let mut labels = vec!["Same as theme"];
        labels.extend(&names);
        let selected = file
            .base_light
            .as_ref()
            .and_then(|b| presets.iter().position(|(id, _)| id == b))
            .map_or(0, |i| i + 1);
        let row = combo("Light Theme", &labels, selected);
        row.set_subtitle("Used while the desktop prefers a light style");
        let (ctx, presets) = (ctx.clone(), presets.clone());
        row.connect_selected_notify(move |row| {
            let chosen = (row.selected() as usize)
                .checked_sub(1)
                .and_then(|i| presets.get(i));
            ctx.theme(|d| match chosen {
                Some((id, _)) => edit::set(d, None, "base-light", id.as_str()),
                None => {
                    edit::remove(d, None, "base-light");
                    if d.get("mode").and_then(|m| m.as_str()) == Some("system") {
                        edit::remove(d, None, "mode");
                    }
                    Ok(())
                }
            });
        });
        group.add(&row);
    }
    page.add(&group);

    // ── Style ──
    let group = new_group("Style", None);
    group.add(&accent_row(ctx, &file, current.as_ref()));
    {
        let row = adw::SpinRow::with_range(0.0, 32.0, 1.0);
        row.set_title("Corner Radius");
        row.set_subtitle("Pixels; 0 gives square corners");
        row.set_value(current.as_ref().map_or(14.0, |t| t.radius));
        let ctx = ctx.clone();
        row.connect_value_notify(move |row| {
            let radius = row.value().round() as i64;
            ctx.theme(|d| edit::set(d, Some("shape"), "radius", radius));
        });
        group.add(&row);
    }
    {
        const DENSITIES: [(Density, &str, &str); 3] = [
            (Density::Compact, "compact", "Compact"),
            (Density::Comfortable, "comfortable", "Comfortable"),
            (Density::Spacious, "spacious", "Spacious"),
        ];
        let density = current.as_ref().map_or(Density::Comfortable, |t| t.density);
        let selected = DENSITIES.iter().position(|(d, _, _)| *d == density);
        let labels: Vec<&str> = DENSITIES.iter().map(|(_, _, l)| *l).collect();
        let row = combo("Density", &labels, selected.unwrap_or(1));
        row.set_subtitle("Spacing around items, rows and panes");
        let ctx = ctx.clone();
        row.connect_selected_notify(move |row| {
            let (_, id, _) = DENSITIES[row.selected() as usize % DENSITIES.len()];
            ctx.theme(|d| edit::set(d, Some("shape"), "density", id));
        });
        group.add(&row);
    }
    page.add(&group);

    // ── Text ──
    let group = new_group("Text", None);
    let desktop_font = gtk4::Settings::default()
        .and_then(|s| s.gtk_font_name())
        // "Cantarell 11" → "Cantarell"
        .and_then(|f| pango::FontDescription::from_string(&f).family())
        .map(|f| f.to_string())
        .unwrap_or_else(|| "sans-serif".into());
    group.add(&font_row(
        ctx,
        "Interface Font",
        "ui",
        "sans-serif",
        file.fonts.ui.as_deref().filter(|f| *f != "system"),
        current
            .as_ref()
            .and_then(|t| t.font_ui.clone())
            .unwrap_or(desktop_font),
    ));
    group.add(&font_row(
        ctx,
        "Monospace Font",
        "mono",
        "monospace",
        file.fonts.mono.as_deref(),
        current
            .as_ref()
            .map_or("monospace".into(), |t| t.font_mono.clone()),
    ));
    {
        let row = adw::SpinRow::with_range(0.5, 2.0, 0.05);
        row.set_digits(2);
        row.set_title("Text Scale");
        row.set_value(current.as_ref().map_or(1.0, |t| t.font_scale));
        let ctx = ctx.clone();
        row.connect_value_notify(move |row| {
            let scale = (row.value() * 100.0).round() / 100.0;
            ctx.theme(|d| edit::set(d, Some("fonts"), "scale", scale));
        });
        group.add(&row);
    }
    page.add(&group);

    // ── Icons (config.toml) ──
    let group = new_group("Icons", None);
    {
        let row = adw::SpinRow::with_range(24.0, 96.0, 4.0);
        row.set_title("Icon Size");
        row.set_value(ctx.state.config.borrow().icon_size as f64);
        let state = ctx.state.clone();
        row.connect_value_notify(move |row| {
            let size = row.value() as i32;
            state.update_config(|cfg| cfg.icon_size = size);
        });
        group.add(&row);
    }
    {
        let names = IconTheme::all_names();
        let current = ctx.state.config.borrow().icon_theme.display_name();
        let row = combo(
            "Icon Style",
            &names,
            names.iter().position(|n| *n == current).unwrap_or(0),
        );
        let state = ctx.state.clone();
        row.connect_selected_notify(move |row| {
            if let Some(name) = IconTheme::all_names().get(row.selected() as usize) {
                state.update_config(|cfg| cfg.icon_theme = IconTheme::from_name(name));
            }
        });
        group.add(&row);
    }
    page.add(&group);

    // ── Files ──
    let group = new_group(
        "Files",
        Some("Everything above is saved to theme.toml. It holds more: per-file-kind colors, every color token, and presets of your own in themes/."),
    );
    group.add(&open_file_row(ctx, "theme.toml", &theme.theme_path()));
    group.add(&open_file_row(ctx, "user.css", &theme.user_css_path()));
    {
        let row = adw::ActionRow::builder()
            .title("Reset to Theme")
            .subtitle("Remove your color, shape and font changes")
            .build();
        let button = suffix_button("Reset…");
        button.add_css_class("destructive-action");
        row.add_suffix(&button);
        let ctx = ctx.clone();
        button.connect_clicked(move |_| confirm_reset(&ctx));
        group.add(&row);
    }
    page.add(&group);
    page
}

fn confirm_reset(ctx: &C) {
    let alert = adw::AlertDialog::new(
        Some("Reset to Theme?"),
        Some("Your accent, shape and font changes are removed from theme.toml. The chosen theme stays."),
    );
    alert.add_responses(&[("cancel", "Cancel"), ("reset", "Reset")]);
    alert.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
    alert.set_close_response("cancel");
    alert.present(ctx.dialog.upgrade().as_ref());
    let ctx = ctx.clone();
    alert.connect_response(None, move |_, response| {
        if response == "reset" && ctx.report(ctx.state.theme.reset_overrides()) {
            // Rebuild so every control shows the theme's own values.
            if let Some(dialog) = ctx.dialog.upgrade() {
                dialog.force_close();
            }
            present_page(&ctx.state, Some("appearance"));
        }
    });
}

fn accent_row(
    ctx: &C,
    file: &crate::theme::ThemeFile,
    current: Option<&crate::theme::Theme>,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title("Accent Color").build();
    let button = gtk4::ColorDialogButton::builder()
        .dialog(&gtk4::ColorDialog::builder().with_alpha(false).build())
        .valign(gtk4::Align::Center)
        .build();
    let resolved_accent = |ctx: &C| {
        ctx.state
            .theme
            .current()
            .ok()
            .and_then(|t| t.colors.get("accent").copied())
            .and_then(|c| gtk4::gdk::RGBA::parse(c.to_string()).ok())
    };
    if let Some(rgba) = current
        .and_then(|t| t.colors.get("accent"))
        .and_then(|c| gtk4::gdk::RGBA::parse(c.to_string()).ok())
    {
        button.set_rgba(&rgba);
    }
    let reset = reset_button("Use the theme’s accent");
    reset.set_visible(file.colors.contains_key("accent"));
    row.add_suffix(&reset);
    row.add_suffix(&button);

    let quiet = Rc::new(Cell::new(false));
    {
        let (ctx, reset, quiet) = (ctx.clone(), reset.clone(), quiet.clone());
        button.connect_rgba_notify(move |b| {
            if quiet.get() {
                return;
            }
            let hex = hex(&b.rgba());
            if ctx.theme(|d| edit::set(d, Some("colors"), "accent", hex.as_str())) {
                reset.set_visible(true);
            }
        });
    }
    {
        let ctx = ctx.clone();
        let button = button.downgrade();
        reset.connect_clicked(move |reset| {
            if ctx.theme(|d| {
                edit::remove(d, Some("colors"), "accent");
                Ok(())
            }) {
                reset.set_visible(false);
                if let (Some(rgba), Some(button)) = (resolved_accent(&ctx), button.upgrade()) {
                    quiet.set(true);
                    button.set_rgba(&rgba);
                    quiet.set(false);
                }
            }
        });
    }
    row
}

fn hex(c: &gtk4::gdk::RGBA) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(c.red()),
        byte(c.green()),
        byte(c.blue())
    )
}

/// A font picker for `[fonts] <key>`; `shown` is the family in effect.
fn font_row(
    ctx: &C,
    title: &str,
    key: &'static str,
    fallback: &'static str,
    overridden: Option<&str>,
    shown: String,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).build();
    let button = gtk4::FontDialogButton::builder()
        .dialog(&gtk4::FontDialog::new())
        .level(gtk4::FontLevel::Family)
        .valign(gtk4::Align::Center)
        .build();
    let family = first_family(&shown);
    button.set_font_desc(&pango::FontDescription::from_string(&family));
    let generic = ["sans-serif", "serif", "monospace", "sans", "system-ui"];
    let installed = generic.contains(&family.to_lowercase().as_str())
        || button
            .pango_context()
            .list_families()
            .iter()
            .any(|f| f.name().eq_ignore_ascii_case(&family));
    if !installed {
        row.set_use_markup(false);
        row.set_subtitle(&format!(
            "{} isn’t installed, so a fallback is used",
            family
        ));
    }
    let reset = reset_button(if key == "ui" {
        "Use the desktop’s font"
    } else {
        "Use the theme’s font"
    });
    reset.set_visible(overridden.is_some());
    row.add_suffix(&reset);
    row.add_suffix(&button);

    {
        let (ctx, reset) = (ctx.clone(), reset.clone());
        button.connect_font_desc_notify(move |b| {
            let Some(family) = b.font_desc().and_then(|d| d.family()) else {
                return;
            };
            // Quoted, so families like "Noto Sans 3" stay valid CSS.
            let value = format!("\"{}\", {}", family.replace('"', ""), fallback);
            if ctx.theme(|d| edit::set(d, Some("fonts"), key, value.as_str())) {
                reset.set_visible(true);
            }
        });
    }
    {
        let ctx = ctx.clone();
        reset.connect_clicked(move |reset| {
            if ctx.theme(|d| {
                edit::remove(d, Some("fonts"), key);
                Ok(())
            }) {
                reset.set_visible(false);
            }
        });
    }
    row
}

/// `"JetBrains Mono", monospace` → `JetBrains Mono`.
fn first_family(css: &str) -> String {
    css.split(',')
        .next()
        .unwrap_or(css)
        .trim()
        .trim_matches(['"', '\''])
        .to_string()
}

// ═══════════════════════════════════════════════
//  Layout — layout.toml
// ═══════════════════════════════════════════════

fn layout_page(ctx: &C) -> adw::PreferencesPage {
    use layout::LAYOUT_FILE;
    let page = page("Layout", "view-dual-symbolic", "layout");
    let layout = ctx.state.layout.borrow().clone();

    // ── Panes ──
    let group = new_group("Panes", None);
    let switch = |title: &str, active: bool, table: &'static str, key: &'static str| {
        let row = adw::SwitchRow::builder()
            .title(title)
            .active(active)
            .build();
        let ctx = ctx.clone();
        row.connect_active_notify(move |row| {
            let on = row.is_active();
            ctx.file(LAYOUT_FILE, |d| edit::set(d, Some(table), key, on));
        });
        row
    };
    let width = |title: &str, value: u32, min: f64, max: f64, table: &'static str| {
        let row = adw::SpinRow::with_range(min, max, 10.0);
        row.set_title(title);
        row.set_value(value as f64);
        let ctx = ctx.clone();
        row.connect_value_notify(move |row| {
            let w = row.value() as i64;
            ctx.file(LAYOUT_FILE, |d| edit::set(d, Some(table), "width", w));
        });
        row
    };
    group.add(&switch(
        "Show Sidebar",
        layout.sidebar.visible,
        "sidebar",
        "visible",
    ));
    group.add(&width(
        "Sidebar Width",
        layout.sidebar.width,
        120.0,
        600.0,
        "sidebar",
    ));
    group.add(&switch(
        "Show Inspector",
        layout.inspector.visible,
        "inspector",
        "visible",
    ));
    {
        let sides = [Side::Left, Side::Right];
        let row = combo(
            "Inspector Position",
            &["Left", "Right"],
            sides
                .iter()
                .position(|s| *s == layout.inspector.position)
                .unwrap_or(1),
        );
        let ctx = ctx.clone();
        row.connect_selected_notify(move |row| {
            let side = sides[row.selected() as usize % 2].id();
            ctx.file(LAYOUT_FILE, |d| {
                edit::set(d, Some("inspector"), "position", side)
            });
        });
        group.add(&row);
    }
    group.add(&width(
        "Inspector Width",
        layout.inspector.width,
        180.0,
        800.0,
        "inspector",
    ));
    page.add(&group);

    // ── Window ──
    let group = new_group("Window", None);
    {
        let labels: Vec<&str> = Decorations::ALL.iter().map(|d| d.title()).collect();
        let row = combo(
            "Title Bar",
            &labels,
            Decorations::ALL
                .iter()
                .position(|d| *d == layout.window.decorations)
                .unwrap_or(0),
        );
        row.set_subtitle(
            "Automatic shows window buttons on GNOME and KDE, and hides them on tiling compositors such as Hyprland or Sway",
        );
        let ctx = ctx.clone();
        row.connect_selected_notify(move |row| {
            let id = Decorations::ALL[row.selected() as usize % Decorations::ALL.len()].id();
            ctx.file(LAYOUT_FILE, |d| {
                edit::set(d, Some("window"), "decorations", id)
            });
        });
        group.add(&row);
    }
    page.add(&group);

    // ── Header bar items ──
    let group = new_group(
        "Header Bar",
        Some("Hidden items stay reachable from the keyboard. Order them in layout.toml."),
    );
    for item in HeaderItem::ALL {
        let shown = layout.header.slots().iter().any(|(_, v)| v.contains(&item));
        let row = adw::SwitchRow::builder()
            .title(item.title())
            .active(shown)
            .build();
        let ctx = ctx.clone();
        row.connect_active_notify(move |row| {
            let on = row.is_active();
            let header = ctx.state.layout.borrow().header.clone();
            ctx.file(LAYOUT_FILE, |d| set_header_item(d, &header, item, on));
        });
        group.add(&row);
    }
    page.add(&group);

    // ── Inspector fields ──
    let group = new_group("Inspector Details", None);
    for field in InspectorField::ALL {
        let row = adw::SwitchRow::builder()
            .title(field.title())
            .active(layout.inspector.fields.contains(&field))
            .build();
        let ctx = ctx.clone();
        row.connect_active_notify(move |row| {
            let on = row.is_active();
            let mut fields = ctx.state.layout.borrow().inspector.fields.clone();
            fields.retain(|f| *f != field);
            if on {
                fields.push(field);
                // Keep the usual order.
                fields.sort_by_key(|f| InspectorField::ALL.iter().position(|a| a == f));
            }
            let ids: Vec<&str> = fields.iter().map(|f| f.id()).collect();
            ctx.file(LAYOUT_FILE, |d| {
                edit::set(d, Some("inspector"), "fields", edit::string_array(&ids))
            });
        });
        group.add(&row);
    }
    page.add(&group);

    let group = new_group("Files", None);
    group.add(&open_file_row(
        ctx,
        "layout.toml",
        &crate::config::persistence::config_dir().join(LAYOUT_FILE),
    ));
    page.add(&group);
    page
}

/// Shows or hides a header item: hiding removes it from its slot, showing
/// appends it to its default slot.
fn set_header_item(
    doc: &mut DocumentMut,
    header: &HeaderLayout,
    item: HeaderItem,
    on: bool,
) -> Result<(), String> {
    let shown = header.slots().iter().any(|(_, v)| v.contains(&item));
    if on == shown {
        return Ok(());
    }
    let home = HeaderLayout::default_slot(item);
    for (slot, items) in header.slots() {
        let mut items = items.clone();
        if on && slot == home {
            items.push(item);
        } else if !on && items.contains(&item) {
            items.retain(|i| *i != item);
        } else {
            continue;
        }
        let ids: Vec<&str> = items.iter().map(|i| i.id()).collect();
        edit::set(doc, Some("header"), slot, edit::string_array(&ids))?;
    }
    Ok(())
}

// ═══════════════════════════════════════════════
//  Behavior — config.toml
// ═══════════════════════════════════════════════

fn behavior_page(ctx: &C) -> adw::PreferencesPage {
    let page = page("Behavior", "preferences-system-symbolic", "behavior");
    let cfg = ctx.state.config();
    let state = &ctx.state;

    let group = new_group("Opening", None);
    {
        let row = combo(
            "Open Items With",
            &["Double click", "Single click"],
            (cfg.open_with == OpenWith::SingleClick) as usize,
        );
        let state = state.clone();
        row.connect_selected_notify(move |row| {
            let value = if row.selected() == 1 {
                OpenWith::SingleClick
            } else {
                OpenWith::DoubleClick
            };
            state.update_config(|cfg| cfg.open_with = value);
        });
        group.add(&row);
    }
    page.add(&group);

    let group = new_group("Folders", None);
    {
        const MODES: [(ViewMode, &str); 4] = [
            (ViewMode::Grid, "Grid"),
            (ViewMode::List, "List"),
            (ViewMode::Tree, "Tree"),
            (ViewMode::Graph, "Graph"),
        ];
        let labels: Vec<&str> = MODES.iter().map(|(_, l)| *l).collect();
        let row = combo(
            "View",
            &labels,
            MODES
                .iter()
                .position(|(m, _)| *m == cfg.view_mode)
                .unwrap_or(0),
        );
        let state = state.clone();
        row.connect_selected_notify(move |row| {
            let (mode, _) = MODES[row.selected() as usize % MODES.len()].clone();
            state.set_view_mode(mode);
        });
        group.add(&row);
    }
    {
        const GROUPS: [(GroupBy, &str); 4] = [
            (GroupBy::None, "None"),
            (GroupBy::Type, "Type"),
            (GroupBy::Date, "Date modified"),
            (GroupBy::Name, "First letter"),
        ];
        let labels: Vec<&str> = GROUPS.iter().map(|(_, l)| *l).collect();
        let row = combo(
            "Group By",
            &labels,
            GROUPS
                .iter()
                .position(|(g, _)| *g == cfg.grouping)
                .unwrap_or(0),
        );
        let state = state.clone();
        row.connect_selected_notify(move |row| {
            let (group, _) = GROUPS[row.selected() as usize % GROUPS.len()].clone();
            state.update_config(|cfg| cfg.grouping = group);
        });
        group.add(&row);
    }
    let toggle = |title: &str,
                  subtitle: Option<&str>,
                  active: bool,
                  set: fn(&mut crate::config::AppConfig, bool)| {
        let row = adw::SwitchRow::builder()
            .title(title)
            .active(active)
            .build();
        if let Some(s) = subtitle {
            row.set_subtitle(s);
        }
        let state = state.clone();
        row.connect_active_notify(move |row| {
            let on = row.is_active();
            state.update_config(|cfg| set(cfg, on));
        });
        row
    };
    group.add(&toggle(
        "Show Hidden Files",
        None,
        cfg.show_hidden,
        |c, v| c.show_hidden = v,
    ));
    group.add(&toggle(
        "Show File Size",
        Some("In list view and on cards"),
        cfg.show_file_size,
        |c, v| c.show_file_size = v,
    ));
    group.add(&toggle(
        "Show Modified Date",
        None,
        cfg.show_modified_date,
        |c, v| c.show_modified_date = v,
    ));
    page.add(&group);
    page
}

// ═══════════════════════════════════════════════
//  Shortcuts — keybindings.toml
// ═══════════════════════════════════════════════

/// Who a shortcut belongs to.
#[derive(Clone, Copy, PartialEq)]
enum Owner {
    Builtin(&'static str),
    Custom(usize),
}

impl Owner {
    fn title(self, state: &AppState) -> String {
        match self {
            Owner::Builtin(name) => keybindings::bindable(name)
                .map(|b| b.title.to_string())
                .unwrap_or_else(|| name.to_string()),
            Owner::Custom(i) => state
                .actions
                .borrow()
                .0
                .get(i)
                .map(|a| format!("“{}”", a.name))
                .unwrap_or_default(),
        }
    }
}

/// `"<Control>a"` and `"<Ctrl>A"` → the same string; `None` if invalid.
fn canonical(accel: &str) -> Option<String> {
    let (key, mods) = gtk4::accelerator_parse(accel)?;
    Some(gtk4::accelerator_name(key, mods).to_string())
}

/// The action currently using `accel`, other than `except`.
fn owner_of(state: &AppState, accel: &str, except: Owner) -> Option<Owner> {
    let accel = canonical(accel)?;
    let is = |a: &String| canonical(a).as_deref() == Some(accel.as_str());
    let keys = state.keybindings.borrow();
    let builtin = BINDABLE
        .iter()
        .map(|b| Owner::Builtin(b.name))
        .find(|o| *o != except && matches!(o, Owner::Builtin(n) if keys.get(n).iter().any(is)));
    builtin.or_else(|| {
        state
            .actions
            .borrow()
            .0
            .iter()
            .enumerate()
            .map(|(i, a)| (Owner::Custom(i), a))
            .find(|(o, a)| *o != except && a.key.as_ref().is_some_and(is))
            .map(|(o, _)| o)
    })
}

/// Takes `accel` away from `owner` (before giving it to someone else).
fn release(ctx: &C, owner: Owner, accel: &str) -> bool {
    let target = canonical(accel);
    match owner {
        Owner::Builtin(name) => {
            let remaining: Vec<String> = ctx
                .state
                .keybindings
                .borrow()
                .get(name)
                .iter()
                .filter(|a| canonical(a) != target)
                .cloned()
                .collect();
            ctx.file(keybindings::KEYBINDINGS_FILE, |d| {
                keybindings::write_keys(d, name, &remaining)
            })
        }
        Owner::Custom(i) => {
            let Some(mut action) = ctx.state.actions.borrow().0.get(i).cloned() else {
                return true;
            };
            action.key = None;
            ctx.file(actions::ACTIONS_FILE, |d| {
                actions::write_action(d, Some(i), &action)
            })
        }
    }
}

enum Recorded {
    Set(String),
    Disable,
    Default,
}

/// Asks for a new shortcut for `owner`. Every other shortcut is off while
/// the dialog is open, so pressing e.g. Ctrl+W records it instead of
/// closing the window.
fn record_shortcut(ctx: &C, owner: Owner, allow_default: bool, done: impl Fn(Recorded) + 'static) {
    let alert = adw::AlertDialog::new(
        Some(&format!("Shortcut for {}", owner.title(&ctx.state))),
        Some("Press a key combination, or Escape to cancel."),
    );
    alert.add_response("cancel", "Cancel");
    alert.add_response("disable", "None");
    if allow_default {
        alert.add_response("default", "Default");
    }
    alert.add_response("set", "Set");
    alert.set_response_appearance("set", adw::ResponseAppearance::Suggested);
    alert.set_response_enabled("set", false);
    alert.set_close_response("cancel");

    let shown = gtk4::ShortcutLabel::builder()
        .disabled_text("…")
        .halign(gtk4::Align::Center)
        .build();
    let note = gtk4::Label::builder()
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .css_classes(["dim-label", "caption"])
        .visible(false)
        .build();
    let extra = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    extra.append(&shown);
    extra.append(&note);
    alert.set_extra_child(Some(&extra));

    let recorded: Rc<RefCell<Option<String>>> = Rc::default();
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let (alert_c, ctx, recorded) = (alert.downgrade(), ctx.clone(), recorded.clone());
        keys.connect_key_pressed(move |keys, key, _, modifiers| {
            use gtk4::gdk::Key;
            let mods = modifiers & gtk4::accelerator_get_default_mod_mask();
            let modifier_only = keys
                .current_event()
                .and_then(|e| e.downcast::<gtk4::gdk::KeyEvent>().ok())
                .is_some_and(|e| e.is_modifier());
            if modifier_only {
                return glib::Propagation::Proceed;
            }
            if mods.is_empty() {
                match key {
                    Key::Escape | Key::Tab | Key::ISO_Left_Tab => {
                        return glib::Propagation::Proceed
                    }
                    // Enter confirms once something was recorded.
                    Key::Return | Key::KP_Enter if recorded.borrow().is_some() => {
                        return glib::Propagation::Proceed
                    }
                    _ => {}
                }
            }
            let accel = gtk4::accelerator_name(key.to_lower(), mods).to_string();
            shown.set_accelerator(&accel);
            match owner_of(&ctx.state, &accel, owner) {
                Some(other) => {
                    note.set_label(&format!(
                        "Used by {} — setting it here removes it there.",
                        other.title(&ctx.state)
                    ));
                    note.set_visible(true);
                }
                None => note.set_visible(false),
            }
            *recorded.borrow_mut() = Some(accel);
            if let Some(alert) = alert_c.upgrade() {
                alert.set_response_enabled("set", true);
                alert.set_default_response(Some("set"));
            }
            glib::Propagation::Stop
        });
    }
    alert.add_controller(keys);

    ctx.state.suspend_shortcuts();
    alert.present(ctx.dialog.upgrade().as_ref());
    let ctx = ctx.clone();
    alert.connect_response(None, move |_, response| {
        ctx.state.apply_shortcuts();
        let choice = match response {
            "set" => match recorded.borrow().clone() {
                Some(accel) => {
                    if let Some(other) = owner_of(&ctx.state, &accel, owner) {
                        if !release(&ctx, other, &accel) {
                            return;
                        }
                    }
                    Recorded::Set(accel)
                }
                None => return,
            },
            "disable" => Recorded::Disable,
            "default" => Recorded::Default,
            _ => return,
        };
        done(choice);
    });
}

fn shortcut_section(name: &str) -> &'static str {
    match name {
        "back" | "forward" | "go-up" | "go-home" | "refresh" | "open-terminal" => "Navigation",
        n if n.starts_with("view-mode")
            || matches!(
                n,
                "cycle-view" | "toggle-hidden" | "toggle-sidebar" | "toggle-inspector"
            ) =>
        {
            "View"
        }
        "show-settings" | "command-palette" | "close-window" | "about" => "Application",
        _ => "Files",
    }
}

fn shortcuts_page(ctx: &C) -> adw::PreferencesPage {
    let page = page(
        "Shortcuts",
        "preferences-desktop-keyboard-shortcuts-symbolic",
        "shortcuts",
    );
    // (name, label, reset button) per row, refreshed after every change
    // since taking a shortcut can change another row.
    type Rows = Rc<RefCell<Vec<(&'static str, gtk4::ShortcutLabel, gtk4::Button)>>>;
    let rows: Rows = Rc::default();
    let refresh: Rc<dyn Fn()> = {
        let (rows, state) = (rows.clone(), ctx.state.clone());
        Rc::new(move || {
            let keys = state.keybindings.borrow();
            for (name, label, reset) in rows.borrow().iter() {
                label.set_accelerator(&keys.get(name).join(" "));
                let default = keybindings::bindable(name).map_or(&[][..], |b| b.defaults);
                reset.set_visible(
                    !keys
                        .get(name)
                        .iter()
                        .map(String::as_str)
                        .eq(default.iter().copied()),
                );
            }
        })
    };

    for section in ["Navigation", "View", "Files", "Application"] {
        let description = (section == "Files")
            .then_some("These work while the file view has the focus, so text fields keep Delete, F2, Escape and Ctrl+A.");
        let group = new_group(section, description);
        for b in BINDABLE
            .iter()
            .filter(|b| shortcut_section(b.name) == section)
        {
            let row = adw::ActionRow::builder()
                .title(b.title)
                .activatable(true)
                .build();
            if b.scope == Scope::View && section != "Files" {
                row.set_subtitle("File view only");
            }
            let label = gtk4::ShortcutLabel::builder()
                .disabled_text("None")
                .valign(gtk4::Align::Center)
                .build();
            let reset = reset_button("Restore the default shortcut");
            row.add_suffix(&reset);
            row.add_suffix(&label);
            rows.borrow_mut().push((b.name, label, reset.clone()));

            let write = {
                let (ctx, refresh) = (ctx.clone(), refresh.clone());
                let name = b.name;
                Rc::new(move |choice: Recorded| {
                    let current = ctx.state.keybindings.borrow().get(name).to_vec();
                    let keys: Vec<String> = match choice {
                        Recorded::Set(accel) => {
                            // Replaces the first shortcut; extra ones stay.
                            let mut keys = current;
                            let canon = canonical(&accel);
                            keys.retain(|k| canonical(k) != canon);
                            match keys.first_mut() {
                                Some(first) => *first = accel,
                                None => keys.push(accel),
                            }
                            keys
                        }
                        Recorded::Disable => vec![],
                        Recorded::Default => keybindings::bindable(name)
                            .map(|b| b.defaults.iter().map(|s| s.to_string()).collect())
                            .unwrap_or_default(),
                    };
                    ctx.file(keybindings::KEYBINDINGS_FILE, |d| {
                        keybindings::write_keys(d, name, &keys)
                    });
                    refresh();
                })
            };
            {
                let (ctx, write) = (ctx.clone(), write.clone());
                let name = b.name;
                row.connect_activated(move |_| {
                    let write = write.clone();
                    record_shortcut(&ctx, Owner::Builtin(name), true, move |c| write(c));
                });
            }
            reset.connect_clicked(move |_| write(Recorded::Default));
            group.add(&row);
        }
        page.add(&group);
    }
    refresh();

    let group = new_group(
        "Files",
        Some("Shortcuts for your own commands are set on the Actions page."),
    );
    group.add(&open_file_row(
        ctx,
        "keybindings.toml",
        &crate::config::persistence::config_dir().join(keybindings::KEYBINDINGS_FILE),
    ));
    page.add(&group);
    page
}

// ═══════════════════════════════════════════════
//  Actions — actions.toml
// ═══════════════════════════════════════════════

fn actions_page(ctx: &C) -> adw::PreferencesPage {
    let page = page("Actions", "system-run-symbolic", "actions");
    let group = new_group(
        "Your Actions",
        Some("Commands in the right-click menu and the command palette. They run without a shell, on the selection or the current folder."),
    );
    let add = suffix_button("Add…");
    group.set_header_suffix(Some(&add));
    page.add(&group);

    let list = Rc::new(ActionList {
        ctx: ctx.clone(),
        group: group.downgrade(),
        rows: RefCell::default(),
    });
    list.fill();
    {
        let list = Rc::downgrade(&list);
        add.connect_clicked(move |_| {
            let Some(list) = list.upgrade() else { return };
            let blank = CustomAction {
                name: String::new(),
                command: String::new(),
                when: When::Any,
                key: None,
                shell: false,
                icon: None,
            };
            edit_action(&list.ctx, None, blank, list.refill());
        });
    }
    // Owned by the page; the rows only point back weakly.
    page.connect_destroy(move |_| {
        let _ = &list;
    });

    let group = new_group(
        "Placeholders",
        Some("{file} — first selected item\n{files} — every selected item\n{name} — first item’s file name\n{dir} — the current folder"),
    );
    group.add(&open_file_row(
        ctx,
        "actions.toml",
        &crate::config::persistence::config_dir().join(actions::ACTIONS_FILE),
    ));
    page.add(&group);
    page
}

/// The "Your Actions" rows, rebuilt after every add / edit / delete.
struct ActionList {
    ctx: C,
    group: glib::WeakRef<adw::PreferencesGroup>,
    rows: RefCell<Vec<gtk4::Widget>>,
}

impl ActionList {
    fn refill(self: &Rc<Self>) -> Rc<dyn Fn()> {
        let list = Rc::downgrade(self);
        Rc::new(move || {
            if let Some(list) = list.upgrade() {
                list.fill();
            }
        })
    }

    fn fill(self: &Rc<Self>) {
        let Some(group) = self.group.upgrade() else {
            return;
        };
        for row in self.rows.borrow_mut().drain(..) {
            group.remove(&row);
        }
        let actions = self.ctx.state.actions.borrow().0.clone();
        if actions.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No actions yet")
                .subtitle("Add one, e.g. “Open Terminal Here” with kitty --directory {dir}")
                .use_markup(false)
                .build();
            group.add(&row);
            self.rows.borrow_mut().push(row.upcast());
        }
        for (i, action) in actions.into_iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(&action.name)
                .subtitle(&action.command)
                .subtitle_lines(1)
                .use_markup(false)
                .activatable(true)
                .build();
            row.add_css_class("property");
            if let Some(key) = &action.key {
                row.add_suffix(
                    &gtk4::ShortcutLabel::builder()
                        .accelerator(key.as_str())
                        .valign(gtk4::Align::Center)
                        .build(),
                );
            }
            row.add_suffix(
                &gtk4::Label::builder()
                    .label(action.when.describe())
                    .css_classes(["dim-label"])
                    .build(),
            );
            row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
            let list = Rc::downgrade(self);
            row.connect_activated(move |_| {
                if let Some(list) = list.upgrade() {
                    edit_action(&list.ctx, Some(i), action.clone(), list.refill());
                }
            });
            group.add(&row);
            self.rows.borrow_mut().push(row.upcast());
        }
    }
}

const WHEN_CHOICES: [&str; 5] = [
    "Always",
    "Nothing selected",
    "Files",
    "Folders",
    "Files with extensions",
];

/// The editor subpage for action `index` (`None` = a new one).
fn edit_action(ctx: &C, index: Option<usize>, action: CustomAction, refill: Rc<dyn Fn()>) {
    let page = adw::PreferencesPage::new();
    let group = new_group("", None);

    let name = adw::EntryRow::builder()
        .title("Name")
        .text(&action.name)
        .build();
    let command = adw::EntryRow::builder()
        .title("Command")
        .text(&action.command)
        .build();
    command.add_css_class("monospace");
    let when = combo(
        "Show For",
        &WHEN_CHOICES,
        match action.when {
            When::Any => 0,
            When::None => 1,
            When::File => 2,
            When::Folder => 3,
            When::Ext(_) => 4,
        },
    );
    let exts = adw::EntryRow::builder()
        .title("Extensions (e.g. png, jpg)")
        .text(match &action.when {
            When::Ext(list) => list.join(", "),
            _ => String::new(),
        })
        .visible(matches!(action.when, When::Ext(_)))
        .build();
    {
        let exts = exts.clone();
        when.connect_selected_notify(move |w| exts.set_visible(w.selected() == 4));
    }
    let shell = adw::SwitchRow::builder()
        .title("Run in a Shell")
        .subtitle("Allows pipes and &&; placeholders are quoted for you")
        .active(action.shell)
        .build();
    let icon = adw::EntryRow::builder()
        .title("Icon Name (optional)")
        .text(action.icon.as_deref().unwrap_or(""))
        .build();

    let key: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(action.key.clone()));
    let key_row = adw::ActionRow::builder()
        .title("Shortcut")
        .activatable(true)
        .build();
    let key_label = gtk4::ShortcutLabel::builder()
        .accelerator(action.key.as_deref().unwrap_or(""))
        .disabled_text("None")
        .valign(gtk4::Align::Center)
        .build();
    key_row.add_suffix(&key_label);
    {
        let (ctx, key) = (ctx.clone(), key.clone());
        let owner = Owner::Custom(index.unwrap_or(usize::MAX));
        key_row.connect_activated(move |_| {
            let (key, key_label) = (key.clone(), key_label.clone());
            record_shortcut(&ctx, owner, false, move |choice| {
                let value = match choice {
                    Recorded::Set(accel) => Some(accel),
                    _ => None,
                };
                key_label.set_accelerator(value.as_deref().unwrap_or(""));
                *key.borrow_mut() = value;
            });
        });
    }

    for row in [
        name.upcast_ref::<gtk4::Widget>(),
        command.upcast_ref(),
        when.upcast_ref(),
        exts.upcast_ref(),
        shell.upcast_ref(),
        key_row.upcast_ref(),
        icon.upcast_ref(),
    ] {
        group.add(row);
    }
    page.add(&group);

    if let Some(i) = index {
        let group = new_group("", None);
        let delete = gtk4::Button::builder()
            .label("Delete Action")
            .halign(gtk4::Align::Center)
            .css_classes(["destructive-action", "pill"])
            .build();
        let (ctx, refill) = (ctx.clone(), refill.clone());
        delete.connect_clicked(move |_| {
            if ctx.file(actions::ACTIONS_FILE, |d| actions::remove_action(d, i)) {
                refill();
                ctx.pop_subpage();
            }
        });
        group.add(&delete);
        page.add(&group);
    }

    let save = gtk4::Button::builder()
        .label(if index.is_some() { "Save" } else { "Add" })
        .css_classes(["suggested-action"])
        .build();
    let header = adw::HeaderBar::new();
    header.pack_end(&save);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&page));
    let nav = adw::NavigationPage::new(
        &toolbar,
        if index.is_some() {
            "Edit Action"
        } else {
            "New Action"
        },
    );

    {
        let (ctx, name) = (ctx.clone(), name.clone());
        save.connect_clicked(move |_| {
            let when_value = match when.selected() {
                1 => When::None,
                2 => When::File,
                3 => When::Folder,
                4 => match When::try_from(format!("ext:{}", exts.text())) {
                    Ok(w) => w,
                    Err(e) => {
                        ctx.report(Err(e));
                        return;
                    }
                },
                _ => When::Any,
            };
            let icon = icon.text().trim().to_string();
            let edited = CustomAction {
                name: name.text().trim().to_string(),
                command: command.text().trim().to_string(),
                when: when_value,
                key: key.borrow().clone(),
                shell: shell.is_active(),
                icon: (!icon.is_empty()).then_some(icon),
            };
            if ctx.file(actions::ACTIONS_FILE, |d| {
                actions::write_action(d, index, &edited)
            }) {
                refill();
                ctx.pop_subpage();
            }
        });
    }
    if let Some(dialog) = ctx.dialog.upgrade() {
        dialog.push_subpage(&nav);
    }
    name.grab_focus();
}

// ═══════════════════════════════════════════════
//  Building blocks
// ═══════════════════════════════════════════════

fn page(title: &str, icon: &str, name: &str) -> adw::PreferencesPage {
    adw::PreferencesPage::builder()
        .title(title)
        .icon_name(icon)
        .name(name)
        .build()
}

fn new_group(title: &str, description: Option<&str>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(title).build();
    if let Some(d) = description {
        group.set_description(Some(&glib::markup_escape_text(d)));
    }
    group
}

fn combo(title: &str, labels: &[&str], selected: usize) -> adw::ComboRow {
    adw::ComboRow::builder()
        .title(title)
        .model(&gtk4::StringList::new(labels))
        .selected(selected as u32)
        .build()
}

fn suffix_button(label: &str) -> gtk4::Button {
    gtk4::Button::builder()
        .label(label)
        .valign(gtk4::Align::Center)
        .build()
}

fn reset_button(tooltip: &str) -> gtk4::Button {
    gtk4::Button::builder()
        .icon_name("edit-undo-symbolic")
        .tooltip_text(tooltip)
        .valign(gtk4::Align::Center)
        .css_classes(["flat"])
        .build()
}

/// A row naming a config file, with a button that opens it.
fn open_file_row(ctx: &C, title: &str, path: &Path) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(path.display().to_string())
        .use_markup(false)
        .build();
    row.add_css_class("property");
    let button = suffix_button("Open");
    let (ctx, path) = (ctx.clone(), path.to_path_buf());
    button.connect_clicked(move |_| {
        let ctx2 = ctx.clone();
        gtk4::FileLauncher::new(Some(&gio::File::for_path(&path))).launch(
            Some(&ctx.state.window),
            gio::Cancellable::NONE,
            move |result| {
                if let Err(e) = result {
                    ctx2.report(Err(format!("Couldn’t open the file: {}", e.message())));
                }
            },
        );
    });
    row.add_suffix(&button);
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_items_toggle_in_their_default_slot() {
        let run = |src: &str, item: HeaderItem, on: bool| {
            let header = layout::LayoutConfig::parse(src).unwrap().header;
            let out = edit::edit_str(src, |d| set_header_item(d, &header, item, on)).unwrap();
            layout::LayoutConfig::parse(&out).unwrap().header
        };
        let hidden = run("", HeaderItem::Up, false);
        assert!(!hidden.start.contains(&HeaderItem::Up));
        assert_eq!(hidden.end, HeaderLayout::default().end);

        // Turning it on again puts it back at the end of `start`.
        let src = "[header]\nstart = [\"back\"]\ncenter = []\nend = [\"menu\"]\n";
        let shown = run(src, HeaderItem::Path, true);
        assert_eq!(shown.center, [HeaderItem::Path]);
        assert_eq!(shown.start, [HeaderItem::Back]);
        // Already shown: nothing moves.
        let same = run(src, HeaderItem::Menu, true);
        assert_eq!(same.end, [HeaderItem::Menu]);
    }

    #[test]
    fn first_family_strips_quotes_and_fallbacks() {
        assert_eq!(
            first_family("\"JetBrains Mono\", monospace"),
            "JetBrains Mono"
        );
        assert_eq!(first_family("Inter"), "Inter");
    }

    #[test]
    fn every_bindable_has_a_section() {
        let sections = ["Navigation", "View", "Files", "Application"];
        for b in BINDABLE {
            assert!(sections.contains(&shortcut_section(b.name)));
        }
        assert_eq!(shortcut_section("view-mode::grid"), "View");
        assert_eq!(shortcut_section("rename-selection"), "Files");
    }
}
