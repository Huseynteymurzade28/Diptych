use crate::config::{layout, persistence, AppConfig, LayoutConfig};
use crate::theme::ThemeManager;
use crate::ui::chrome::Chrome;
use crate::ui::state::{AppState, StateWidgets};
use crate::ui::{context_menu, hamburger, inspector, sidebar};
use gtk4::prelude::*;
use gtk4::{Box, Orientation, ScrolledWindow};
use std::path::PathBuf;

// ═══════════════════════════════════════════════
//  Main Window Assembly
// ═══════════════════════════════════════════════

pub fn build(app: &adw::Application) {
    let config = AppConfig::load();
    let config_dir = persistence::config_dir();

    // ── Theme (theme.toml + user.css, hot-reloaded) ──
    let theme = ThemeManager::new(&config_dir, &config.theme);

    // ── Layout (layout.toml, hot-reloaded) ──
    if let Err(e) = layout::seed(&config_dir) {
        eprintln!("[layout] Could not create {}: {}", layout::LAYOUT_FILE, e);
    }
    let layout = LayoutConfig::load(&config_dir).unwrap_or_else(|e| {
        eprintln!("[layout] {}: {} (using defaults)", layout::LAYOUT_FILE, e);
        LayoutConfig::default()
    });

    let start_path = dirs::home_dir()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/"));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Diptych")
        .default_width(config.window_width)
        .default_height(config.window_height)
        // Small enough for a quarter-screen tile.
        .width_request(360)
        .height_request(300)
        .build();

    // ── Panes ──
    let places = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .margin_start(6)
        .margin_end(6)
        .build();
    let sidebar_widget = sidebar::build_sidebar(&places);

    let content_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .margin_top(8)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(8)
        .build();
    let content_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .hexpand(true)
        .child(&content_box)
        .css_classes(["content-view"])
        .build();

    let inspector_pane = inspector::build_pane();
    let inspector_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&inspector_pane)
        .css_classes(["inspector"])
        .build();

    let chrome = Chrome::new(
        &window,
        &sidebar_widget,
        &content_scroll,
        &inspector_scroll,
        &hamburger::build_hamburger_menu(),
    );
    chrome.apply(&layout);

    // ═══════════════════════════════════════════
    //  Shared state
    // ═══════════════════════════════════════════

    let state = AppState::new(
        config,
        layout,
        start_path,
        StateWidgets {
            window: window.clone(),
            theme,
            chrome,
            content_scroll,
            content_box: content_box.clone(),
            places,
            inspector: inspector_pane,
        },
    );
    sidebar::bind_places(&state);
    sidebar::setup_creation_popover(&state);

    // Save window size on close
    {
        let state = state.clone();
        window.connect_close_request(move |w| {
            let mut cfg = state.config.borrow_mut();
            cfg.window_width = w.width();
            cfg.window_height = w.height();
            cfg.save();
            glib::Propagation::Proceed
        });
    }

    // Right-click on empty content area
    context_menu::attach_background_context_menu(&content_box, &state);

    // Left-click on empty space clears the selection. Item buttons claim
    // their own clicks, so this only fires between/below items.
    {
        let gesture = gtk4::GestureClick::builder().button(1).build();
        let state_c = state.clone();
        gesture.connect_pressed(move |_, _, _, _| state_c.clear_selection());
        state.content_scroll.add_controller(gesture);
    }

    // Hot-reload layout.toml; the watch lives as long as the window.
    let watch = {
        let weak = std::rc::Rc::downgrade(&state);
        crate::config::watch::watch(
            &[config_dir],
            |name| name == std::path::Path::new(layout::LAYOUT_FILE),
            move || {
                if let Some(state) = weak.upgrade() {
                    state.reload_layout();
                }
            },
        )
    };
    window.connect_destroy(move |_| {
        let _ = &watch;
    });

    state.refresh();
    window.present();
    crate::ui::snapshot::schedule_if_requested(window.upcast_ref());
}
