use crate::config::AppConfig;
use crate::filesystem;
use crate::thumbnail;
use crate::ui::widgets::icon::{icon_css_class, icon_for_entry_themed};
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Tree View — Hierarchical File Browser
// ═══════════════════════════════════════════════
//
// A modern, cozy tree view with:
//   • Smooth indentation with subtle guide lines
//   • Animated disclosure arrows (▶ / ▼)
//   • Rounded hover highlights
//   • Directory item count badges
//   • File type color coding
//   • Selected item accent highlight

/// Indentation per nesting level (pixels).
const INDENT_PX: i32 = 20;

/// Maximum recursive depth to prevent runaway expansion.
const MAX_DEPTH: u32 = 12;

/// Builds the full tree view starting from `root_path`.
pub fn build_tree_view(
    root_path: Rc<RefCell<PathBuf>>,
    config: Rc<RefCell<AppConfig>>,
    inspector_info: &Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    on_navigate: Rc<dyn Fn(PathBuf)>,
) -> Box {
    let container = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .margin_top(4)
        .margin_bottom(8)
        .css_classes(vec!["tree-view-container".to_string()])
        .build();

    let expanded: Rc<RefCell<HashSet<PathBuf>>> = Rc::new(RefCell::new(HashSet::new()));

    // Expand the root itself by default
    {
        let root = root_path.borrow().clone();
        expanded.borrow_mut().insert(root);
    }

    {
        let root = root_path.borrow().clone();
        render_tree(
            &container,
            &root,
            0,
            expanded.clone(),
            root_path.clone(),
            config.clone(),
            inspector_info,
            selected_file_path.clone(),
            on_navigate.clone(),
        );
    }

    container
}

/// Recursively renders one level of the tree.
fn render_tree(
    container: &Box,
    dir_path: &Path,
    depth: u32,
    expanded: Rc<RefCell<HashSet<PathBuf>>>,
    root_path: Rc<RefCell<PathBuf>>,
    config: Rc<RefCell<AppConfig>>,
    inspector_info: &Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    on_navigate: Rc<dyn Fn(PathBuf)>,
) {
    if depth > MAX_DEPTH {
        return;
    }

    let cfg = config.borrow().clone();
    let entries = filesystem::list_directory(dir_path, cfg.show_hidden);

    if entries.is_empty() && depth > 0 {
        // Show a subtle "empty" hint for expanded empty directories
        let indent = (depth as i32) * INDENT_PX + 30;
        let empty_label = Label::builder()
            .label("empty")
            .halign(Align::Start)
            .margin_start(indent)
            .css_classes(vec!["tree-empty-hint".to_string()])
            .build();
        container.append(&empty_label);
        return;
    }

    for entry in &entries {
        let indent = (depth as i32) * INDENT_PX;

        // ── Outer row wrapper with guide-line indentation ──
        let outer = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .build();

        // Guide line area: one thin bar per depth level
        if depth > 0 {
            let guides = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(0)
                .width_request(indent)
                .build();

            for i in 0..depth {
                let guide = Box::builder()
                    .width_request(INDENT_PX)
                    .vexpand(true)
                    .build();
                // Only the last guide before content gets the visible line
                if i == depth - 1 {
                    guide.add_css_class("tree-guide-line");
                } else {
                    guide.add_css_class("tree-guide-spacer");
                }
                guides.append(&guide);
            }
            outer.append(&guides);
        }

        // ── Inner content row ──
        let row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .hexpand(true)
            .valign(Align::Center)
            .build();

        // ── Disclosure arrow (directories only) ──
        if entry.is_dir {
            let is_open = expanded.borrow().contains(&entry.path);
            let arrow_label = if is_open { "▾" } else { "▸" };

            let arrow_btn = Button::builder()
                .label(arrow_label)
                .has_frame(false)
                .css_classes(vec!["tree-arrow".to_string()])
                .build();

            if is_open {
                arrow_btn.add_css_class("tree-arrow-open");
            }

            // Toggle expand/collapse
            let entry_path = entry.path.clone();
            let expanded_c = expanded.clone();
            let container_c = container.clone();
            let root_c = root_path.clone();
            let config_c = config.clone();
            let info_c = inspector_info.clone();
            let sel_c = selected_file_path.clone();
            let nav_c = on_navigate.clone();

            arrow_btn.connect_clicked(move |_| {
                {
                    let mut set = expanded_c.borrow_mut();
                    if set.contains(&entry_path) {
                        set.remove(&entry_path);
                    } else {
                        set.insert(entry_path.clone());
                    }
                }
                rebuild_tree(
                    &container_c,
                    expanded_c.clone(),
                    root_c.clone(),
                    config_c.clone(),
                    &info_c,
                    sel_c.clone(),
                    nav_c.clone(),
                );
            });

            row.append(&arrow_btn);
        } else {
            // Dot spacer for files — aligns with arrows
            let dot = Label::builder()
                .label("·")
                .width_request(22)
                .halign(Align::Center)
                .css_classes(vec!["tree-file-dot".to_string()])
                .build();
            row.append(&dot);
        }

        // ── Icon ──
        let ext = entry.extension.to_lowercase();
        let has_thumb = !entry.is_dir && thumbnail::supports_thumbnail(&ext);
        let icon_sz = 18;

        let icon: Image = if has_thumb {
            thumbnail::request_thumbnail(&entry.path, icon_sz)
        } else {
            let icon_name = icon_for_entry_themed(entry, &cfg.icon_theme);
            let mut classes = vec!["tree-icon".to_string()];
            if cfg.icon_theme == crate::config::IconTheme::Colorful {
                classes.push(icon_css_class(entry).to_string());
            }
            if entry.is_dir {
                classes.push("tree-icon-folder".to_string());
            }
            Image::builder()
                .icon_name(icon_name)
                .pixel_size(icon_sz)
                .css_classes(classes)
                .build()
        };
        row.append(&icon);

        // ── Name label ──
        let name_css = if entry.is_dir {
            vec!["tree-name".to_string(), "tree-name-dir".to_string()]
        } else {
            vec!["tree-name".to_string(), "tree-name-file".to_string()]
        };

        let name_label = Label::builder()
            .label(&entry.name)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .css_classes(name_css)
            .build();
        row.append(&name_label);

        // ── Badges / Metadata ──
        if entry.is_dir {
            // Show child count badge for directories
            if let Ok(rd) = std::fs::read_dir(&entry.path) {
                let count = rd
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        cfg.show_hidden
                            || !e.file_name().to_string_lossy().starts_with('.')
                    })
                    .count();
                if count > 0 {
                    let badge = Label::builder()
                        .label(&format!("{}", count))
                        .halign(Align::End)
                        .css_classes(vec!["tree-badge".to_string()])
                        .build();
                    row.append(&badge);
                }
            }
        } else if cfg.show_file_size {
            let size_label = Label::builder()
                .label(&entry.size_display())
                .halign(Align::End)
                .css_classes(vec!["tree-meta".to_string()])
                .build();
            row.append(&size_label);
        }

        // ── Wrap in button for click handling ──
        outer.append(&row);

        let row_btn = Button::builder()
            .child(&outer)
            .has_frame(false)
            .css_classes(vec!["tree-row-btn".to_string()])
            .build();

        // Highlight selected item
        {
            let sel_path = selected_file_path.borrow();
            if sel_path.as_ref() == Some(&entry.path) {
                row_btn.add_css_class("tree-row-selected");
            }
        }

        // ── Click handler ──
        {
            let entry_path = entry.path.clone();
            let is_dir = entry.is_dir;
            let name = entry.name.clone();
            let size_disp = entry.size_display();
            let mod_disp = entry.modified_display();
            let info_c = inspector_info.clone();
            let sel_c = selected_file_path.clone();
            let nav_c = on_navigate.clone();
            let expanded_c = expanded.clone();
            let container_c = container.clone();
            let root_c = root_path.clone();
            let config_c = config.clone();

            row_btn.connect_clicked(move |_| {
                if is_dir {
                    // Toggle expansion in-place (don't navigate away)
                    {
                        let mut set = expanded_c.borrow_mut();
                        if set.contains(&entry_path) {
                            set.remove(&entry_path);
                        } else {
                            set.insert(entry_path.clone());
                        }
                    }
                    rebuild_tree(
                        &container_c,
                        expanded_c.clone(),
                        root_c.clone(),
                        config_c.clone(),
                        &info_c,
                        sel_c.clone(),
                        nav_c.clone(),
                    );
                } else {
                    info_c.set_label(&format!(
                        "{}  •  {}  •  {}",
                        name, size_disp, mod_disp
                    ));
                    *sel_c.borrow_mut() = Some(entry_path.clone());
                    if let Err(e) = open::that(&entry_path) {
                        eprintln!("Failed to open file: {}", e);
                    }
                }
            });
        }

        container.append(&row_btn);

        // ── Recurse into expanded directories ──
        if entry.is_dir && expanded.borrow().contains(&entry.path) {
            render_tree(
                container,
                &entry.path,
                depth + 1,
                expanded.clone(),
                root_path.clone(),
                config.clone(),
                inspector_info,
                selected_file_path.clone(),
                on_navigate.clone(),
            );
        }
    }
}

/// Clears and re-renders the full tree (called after expand/collapse toggle).
fn rebuild_tree(
    container: &Box,
    expanded: Rc<RefCell<HashSet<PathBuf>>>,
    root_path: Rc<RefCell<PathBuf>>,
    config: Rc<RefCell<AppConfig>>,
    inspector_info: &Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    on_navigate: Rc<dyn Fn(PathBuf)>,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let root = root_path.borrow().clone();
    render_tree(
        container,
        &root,
        0,
        expanded,
        root_path,
        config,
        inspector_info,
        selected_file_path,
        on_navigate,
    );
}
