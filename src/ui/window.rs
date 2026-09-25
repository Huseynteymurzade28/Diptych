use crate::config::AppConfig;
use crate::core::Theme;
use crate::ui::state::{AppState, StateWidgets};
use crate::ui::{context_menu, hamburger, inspector, sidebar};
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, CssProvider, Label, Orientation, Paned,
    ScrolledWindow,
};
use std::path::PathBuf;

// ═══════════════════════════════════════════════
//  Main Window Assembly
// ═══════════════════════════════════════════════

pub fn build(app: &Application) {
    // ── Load persisted config ──
    let config = AppConfig::load();

    let start_path = dirs::home_dir()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/"));

    // ── Theme setup ──
    let css_provider = CssProvider::new();
    css_provider.load_from_data(&Theme::from_name(&config.theme).to_css());
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // ── Window ──
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Diptych")
        .default_width(config.window_width)
        .default_height(config.window_height)
        .build();

    // ═══════════════════════════════════════════
    //  Layout: Paned  [Sidebar | Content+Header]
    // ═══════════════════════════════════════════

    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .position(220)
        .build();

    // ── Right side: header + content + inspector ──
    let right_vbox = Box::builder()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .build();

    // Header bar
    let header_bar = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .css_classes(vec!["header-bar".to_string()])
        .build();

    let go_up_btn = Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text("Go Up")
        .action_name("win.go-up")
        .css_classes(vec!["toolbar-btn".to_string()])
        .build();

    let breadcrumb_label = Label::builder()
        .label("~")
        .css_classes(vec!["breadcrumb-label-active".to_string()])
        .halign(Align::Start)
        .hexpand(true)
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::Start)
        .build();

    let view_toggle_btn = Button::builder()
        .tooltip_text("Toggle View Mode (Grid / List / Graph / Tree)")
        .action_name("win.cycle-view")
        .css_classes(vec!["toolbar-btn".to_string()])
        .build();

    header_bar.append(&go_up_btn);
    header_bar.append(&breadcrumb_label);
    header_bar.append(&view_toggle_btn);
    header_bar.append(&hamburger::build_hamburger_menu());

    right_vbox.append(&header_bar);

    // Content area
    let content_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .margin_top(8)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(8)
        .build();

    let content_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&content_box)
        .build();

    right_vbox.append(&content_scroll);

    // Inspector bar
    let (inspector_bar, inspector_info) = inspector::build_inspector_bar();
    right_vbox.append(&inspector_bar);

    // Sidebar file list (filled by `sidebar::refresh_sidebar`)
    let nav_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .margin_start(4)
        .margin_end(4)
        .build();

    // ═══════════════════════════════════════════
    //  Shared state
    // ═══════════════════════════════════════════

    let state = AppState::new(
        config,
        start_path,
        StateWidgets {
            window: window.clone(),
            css_provider,
            content_scroll,
            content_box: content_box.clone(),
            nav_box,
            breadcrumb: breadcrumb_label,
            inspector_info,
            view_toggle_btn,
        },
    );

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

    // Assemble paned
    paned.set_start_child(Some(&sidebar::build_sidebar(&state)));
    paned.set_end_child(Some(&right_vbox));
    window.set_child(Some(&paned));

    // Right-click on empty content area
    context_menu::attach_background_context_menu(&content_box, &state);

    state.refresh();
    window.present();
}
