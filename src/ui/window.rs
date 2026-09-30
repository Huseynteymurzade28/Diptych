use crate::config::{
    actions, keybindings, layout, persistence, Actions, AppConfig, Keybindings, LayoutConfig,
};
use crate::theme::ThemeManager;
use crate::ui::chrome::Chrome;
use crate::ui::file_view::FileView;
use crate::ui::state::{AppState, StateWidgets};
use crate::ui::{context_menu, hamburger, inspector, sidebar};
use gtk4::prelude::*;
use gtk4::{Box, Orientation, ScrolledWindow};
use std::path::PathBuf;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Main Window Assembly
// ═══════════════════════════════════════════════

thread_local! {
    /// One theme for every window: its CSS providers are display-wide.
    static THEME: std::cell::OnceCell<Rc<ThemeManager>> = const { std::cell::OnceCell::new() };
}

/// A new window at the home folder (`activate`).
pub fn build(app: &adw::Application) {
    open(app, None, vec![], None);
}

/// A new window showing `dir` (home if `None`) with `select` selected.
/// `startup_id` is the activation token of whoever asked (D-Bus callers),
/// so the compositor lets the window take focus.
pub fn open(
    app: &adw::Application,
    dir: Option<PathBuf>,
    select: Vec<PathBuf>,
    startup_id: Option<&str>,
) {
    let config = AppConfig::load();
    let config_dir = persistence::config_dir();

    // ── Theme (theme.toml + user.css, hot-reloaded) ──
    let theme = THEME.with(|t| {
        t.get_or_init(|| ThemeManager::new(&config_dir, &config.theme))
            .clone()
    });

    // ── Layout (layout.toml, hot-reloaded) ──
    if let Err(e) = layout::seed(&config_dir) {
        eprintln!("[layout] Could not create {}: {}", layout::LAYOUT_FILE, e);
    }
    let layout = LayoutConfig::load(&config_dir).unwrap_or_else(|e| {
        eprintln!("[layout] {}: {} (using defaults)", layout::LAYOUT_FILE, e);
        LayoutConfig::default()
    });

    // ── Shortcuts and custom actions (hot-reloaded) ──
    // Load errors are shown as toasts once the window exists.
    let mut load_errors = vec![];
    if let Err(e) = keybindings::seed(&config_dir) {
        eprintln!(
            "[keys] Could not create {}: {}",
            keybindings::KEYBINDINGS_FILE,
            e
        );
    }
    if let Err(e) = actions::seed(&config_dir) {
        eprintln!(
            "[actions] Could not create {}: {}",
            actions::ACTIONS_FILE,
            e
        );
    }
    let keys = Keybindings::load(&config_dir).unwrap_or_else(|e| {
        load_errors.push((keybindings::KEYBINDINGS_FILE, e));
        Keybindings::default()
    });
    let custom_actions = Actions::load(&config_dir).unwrap_or_else(|e| {
        load_errors.push((actions::ACTIONS_FILE, e));
        Actions::default()
    });

    let start_path = dir
        .filter(|d| d.is_dir())
        .or_else(dirs::home_dir)
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
        .build();

    // Grid / list: the virtualized file view. Tree, graph and settings:
    // `content_scroll`.
    let file_view = FileView::new();
    let content_stack = gtk4::Stack::builder().css_classes(["content-view"]).build();
    content_stack.add_named(&file_view.widget, Some("files"));
    content_stack.add_named(&content_scroll, Some("other"));

    let inspector_pane = inspector::build_pane();
    let inspector_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&inspector_pane)
        .css_classes(["inspector"])
        .build();

    let chrome = Chrome::new(
        &window,
        &sidebar_widget,
        &content_stack,
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
        keys,
        custom_actions,
        start_path,
        StateWidgets {
            window: window.clone(),
            theme,
            chrome,
            content_stack,
            file_view: file_view.clone(),
            content_scroll,
            content_box: content_box.clone(),
            places,
            inspector: inspector_pane,
        },
    );
    file_view.bind(&state);
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

    // Tree view: left-click on empty space clears the selection. Item
    // buttons claim their own clicks, so this only fires between/below items.
    {
        let gesture = gtk4::GestureClick::builder().button(1).build();
        let state_c = state.clone();
        gesture.connect_pressed(move |_, _, _, _| state_c.clear_selection());
        state.content_scroll.add_controller(gesture);
    }

    for (file, e) in &load_errors {
        state.config_status(file, Some(e));
    }

    // Hot-reload the config files; the watches live as long as the window.
    let watch = |file: &'static str, reload: fn(&std::rc::Rc<AppState>)| {
        let weak = std::rc::Rc::downgrade(&state);
        crate::config::watch::watch(
            std::slice::from_ref(&config_dir),
            move |name| name == std::path::Path::new(file),
            move || {
                if let Some(state) = weak.upgrade() {
                    reload(&state);
                }
            },
        )
    };
    let watches = [
        watch(layout::LAYOUT_FILE, |s| s.reload_layout()),
        watch(keybindings::KEYBINDINGS_FILE, |s| s.reload_keybindings()),
        watch(actions::ACTIONS_FILE, |s| s.reload_actions()),
    ];
    // Bookmarks added in Nautilus or the file chooser show up here too.
    let bookmarks = {
        let weak = std::rc::Rc::downgrade(&state);
        let file = crate::integration::bookmarks::file();
        let dir = file.parent().map(|d| d.to_path_buf()).unwrap_or_default();
        crate::config::watch::watch(
            &[dir],
            move |name| Some(name.as_os_str()) == file.file_name(),
            move || {
                if let Some(state) = weak.upgrade() {
                    sidebar::bind_places(&state);
                }
            },
        )
    };
    window.connect_destroy(move |_| {
        let _ = (&watches, &bookmarks);
    });

    state.preselect(select);
    state.refresh();
    if let Some(id) = startup_id.filter(|id| !id.is_empty()) {
        window.set_startup_id(id);
    }
    window.present();
    crate::ui::snapshot::schedule_if_requested(window.upcast_ref());
}
