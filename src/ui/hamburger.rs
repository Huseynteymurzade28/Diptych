use gtk4::prelude::*;
use gtk4::{MenuButton, PopoverMenu};

// ═══════════════════════════════════════════════
//  Hamburger Menu (☰)
// ═══════════════════════════════════════════════
//
// Top-right menu backed by a `gio::Menu` model. Every item is a
// `win.*` action (see `state::install_actions`), so the same
// behavior is reachable from buttons and, later, keyboard shortcuts.

pub fn build_hamburger_menu() -> MenuButton {
    let menu = gio::Menu::new();

    let view_section = gio::Menu::new();
    view_section.append(Some("Show Hidden Files"), Some("win.toggle-hidden"));
    view_section.append(Some("Refresh"), Some("win.refresh"));
    menu.append_section(None, &view_section);

    let app_section = gio::Menu::new();
    app_section.append(Some("Settings"), Some("win.show-settings"));
    app_section.append(Some("About Diptych"), Some("win.about"));
    menu.append_section(None, &app_section);

    let popover = PopoverMenu::from_model(Some(&menu));
    popover.add_css_class("context-menu");

    MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Menu")
        .popover(&popover)
        .css_classes(vec!["toolbar-btn".to_string()])
        .build()
}
