use crate::config::AppConfig;
use crate::ui::content::refresh_content;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Label, MenuButton, Orientation, Popover, Separator};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Hamburger Menu (☰)
// ═══════════════════════════════════════════════
//
// Top-right hamburger button that opens a popover with quick actions:
//   • Open Settings Panel
//   • Toggle Hidden Files
//   • About

/// Builds the hamburger menu button and returns it.
/// `on_open_settings` is called when the user clicks "Settings".
pub fn build_hamburger_menu(
    config: Rc<RefCell<AppConfig>>,
    current_path: Rc<RefCell<PathBuf>>,
    content_box: Box,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    on_open_settings: Rc<dyn Fn()>,
) -> MenuButton {
    let popover = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .build();

    let menu_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(6)
        .margin_end(6)
        .build();

    // ── App title / branding ──
    let title_label = Label::builder()
        .label("Diptych")
        .css_classes(vec!["hamburger-title".to_string()])
        .halign(Align::Start)
        .margin_start(8)
        .margin_bottom(4)
        .build();
    menu_box.append(&title_label);

    let sep1 = Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    menu_box.append(&sep1);

    // ── Settings button ──
    let settings_btn = hamburger_item("emblem-system-symbolic", "Settings");
    menu_box.append(&settings_btn);

    // ── Toggle hidden files ──
    let hidden_label = if config.borrow().show_hidden {
        "Hide Hidden Files"
    } else {
        "Show Hidden Files"
    };
    let hidden_btn = hamburger_item("view-reveal-symbolic", hidden_label);
    menu_box.append(&hidden_btn);

    let sep2 = Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    menu_box.append(&sep2);

    // ── About ──
    let about_btn = hamburger_item("help-about-symbolic", "About Diptych");
    menu_box.append(&about_btn);

    popover.set_child(Some(&menu_box));

    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Menu")
        .popover(&popover)
        .css_classes(vec!["toolbar-btn".to_string()])
        .build();

    // Wire: Settings
    {
        let popover_c = popover.clone();
        let on_settings = on_open_settings.clone();
        settings_btn.connect_clicked(move |_| {
            popover_c.popdown();
            on_settings();
        });
    }

    // Wire: Toggle hidden files
    {
        let popover_c = popover.clone();
        let config_c = config.clone();
        let cp = current_path.clone();
        let cb = content_box.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let hidden_btn_c = hidden_btn.clone();

        hidden_btn.connect_clicked(move |_| {
            {
                let mut cfg = config_c.borrow_mut();
                cfg.show_hidden = !cfg.show_hidden;
                cfg.save();
            }
            // Update button label
            let new_label = if config_c.borrow().show_hidden {
                "Hide Hidden Files"
            } else {
                "Show Hidden Files"
            };
            if let Some(child) = hidden_btn_c.child() {
                if let Some(hbox) = child.downcast_ref::<Box>() {
                    // Second child is the label
                    if let Some(lbl_widget) = hbox.first_child().and_then(|c| c.next_sibling()) {
                        if let Some(lbl) = lbl_widget.downcast_ref::<Label>() {
                            lbl.set_label(new_label);
                        }
                    }
                }
            }
            popover_c.popdown();
            refresh_content(&cb, cp.clone(), &info, sel.clone(), config_c.clone());
        });
    }

    // Wire: About
    {
        let popover_c = popover.clone();
        let menu_btn_c = menu_button.clone();
        about_btn.connect_clicked(move |_| {
            popover_c.popdown();
            // Show a proper About dialog
            if let Some(root) = menu_btn_c.root() {
                if let Some(win) = root.downcast_ref::<gtk4::Window>() {
                    let about = gtk4::AboutDialog::builder()
                        .transient_for(win)
                        .modal(true)
                        .program_name("Diptych")
                        .version("0.1.0")
                        .comments("A modern GTK4 file manager built with Rust.")
                        .website("https://github.com/flear/diptych")
                        .license_type(gtk4::License::MitX11)
                        .build();
                    about.present();
                }
            }
        });
    }

    menu_button
}

// ─── Helper ───

fn hamburger_item(icon_name: &str, label_text: &str) -> Button {
    let hbox = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();

    let icon = gtk4::Image::builder()
        .icon_name(icon_name)
        .pixel_size(16)
        .build();

    let label = Label::builder()
        .label(label_text)
        .xalign(0.0)
        .hexpand(true)
        .build();

    hbox.append(&icon);
    hbox.append(&label);

    Button::builder()
        .child(&hbox)
        .has_frame(false)
        .css_classes(vec!["context-menu-item".to_string()])
        .build()
}
