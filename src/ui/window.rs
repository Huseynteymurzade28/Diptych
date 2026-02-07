use crate::config::{AppConfig, ViewMode};
use crate::core::Theme;
use crate::ui::{content, inspector, sidebar};
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, CssProvider, Label, Orientation, Paned,
    ScrolledWindow, StyleContext,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Main Window Assembly
// ═══════════════════════════════════════════════

pub fn build(app: &Application) {
    // ── Load persisted config ──
    let config = Rc::new(RefCell::new(AppConfig::load()));

    let start_path = dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    let current_path = Rc::new(RefCell::new(start_path));
    let selected_file_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    // ── Theme setup ──
    let css_provider = CssProvider::new();
    let theme = Theme::from_name(&config.borrow().theme);
    css_provider.load_from_data(&theme.to_css());
    if let Some(display) = gtk4::gdk::Display::default() {
        #[allow(deprecated)]
        StyleContext::add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // ── Window ──
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Diptych")
        .default_width(config.borrow().window_width)
        .default_height(config.borrow().window_height)
        .build();

    // Save window size on close
    {
        let config_c = config.clone();
        window.connect_close_request(move |w| {
            let (width, height) = (w.width(), w.height());
            let mut cfg = config_c.borrow_mut();
            cfg.window_width = width;
            cfg.window_height = height;
            cfg.save();
            glib::Propagation::Proceed
        });
    }

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
        .icon_name("view-grid-symbolic")
        .tooltip_text("Toggle View Mode")
        .css_classes(vec!["toolbar-btn".to_string()])
        .build();

    header_bar.append(&go_up_btn);
    header_bar.append(&breadcrumb_label);
    header_bar.append(&view_toggle_btn);
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

    // ── Left side: Sidebar ──
    let (sidebar_widget, nav_box) = sidebar::build_sidebar(
        current_path.clone(),
        selected_file_path.clone(),
        config.clone(),
        content_box.clone(),
        content_scroll,
        breadcrumb_label.clone(),
        inspector_info.clone(),
        window.clone(),
        css_provider,
    );

    // Assemble paned
    paned.set_start_child(Some(&sidebar_widget));
    paned.set_end_child(Some(&right_vbox));
    window.set_child(Some(&paned));

    // ═══════════════════════════════════════════
    //  Go Up Button
    // ═══════════════════════════════════════════
    {
        let current_path_c = current_path.clone();
        let nav_box_c = nav_box.clone();
        let content_box_c = content_box.clone();
        let window_c = window.clone();
        let breadcrumb_c = breadcrumb_label.clone();
        let inspector_info_c = inspector_info.clone();
        let selected_c = selected_file_path.clone();
        let config_c = config.clone();

        go_up_btn.connect_clicked(move |_| {
            let parent = {
                let p = current_path_c.borrow();
                p.parent().map(|pp| pp.to_path_buf())
            };
            if let Some(parent_path) = parent {
                *current_path_c.borrow_mut() = parent_path;
                sidebar::refresh_all(
                    &nav_box_c,
                    &content_box_c,
                    current_path_c.clone(),
                    &window_c,
                    &breadcrumb_c,
                    &inspector_info_c,
                    selected_c.clone(),
                    config_c.clone(),
                );
            }
        });
    }

    // ═══════════════════════════════════════════
    //  View Mode Toggle
    // ═══════════════════════════════════════════
    {
        let config_c = config.clone();
        let content_box_c = content_box.clone();
        let current_path_c = current_path.clone();
        let inspector_info_c = inspector_info.clone();
        let selected_c = selected_file_path.clone();
        let view_btn_c = view_toggle_btn.clone();

        view_toggle_btn.connect_clicked(move |_| {
            {
                let mut cfg = config_c.borrow_mut();
                cfg.view_mode = match cfg.view_mode {
                    ViewMode::Grid => ViewMode::List,
                    ViewMode::List => ViewMode::Grid,
                };
                cfg.save();
            }
            let icon = match config_c.borrow().view_mode {
                ViewMode::Grid => "view-grid-symbolic",
                ViewMode::List => "view-list-symbolic",
            };
            view_btn_c.set_icon_name(icon);

            content::refresh_content(
                &content_box_c,
                current_path_c.clone(),
                &inspector_info_c,
                selected_c.clone(),
                config_c.clone(),
            );
        });
    }

    // ═══════════════════════════════════════════
    //  Initial Render
    // ═══════════════════════════════════════════
    {
        let icon = match config.borrow().view_mode {
            ViewMode::Grid => "view-grid-symbolic",
            ViewMode::List => "view-list-symbolic",
        };
        view_toggle_btn.set_icon_name(icon);
    }

    sidebar::refresh_all(
        &nav_box,
        &content_box,
        current_path,
        &window,
        &breadcrumb_label,
        &inspector_info,
        selected_file_path,
        config,
    );

    window.present();
}
