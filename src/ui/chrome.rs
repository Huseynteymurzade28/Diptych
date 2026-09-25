use crate::config::layout::{auto_decorations, Decorations, HeaderItem, LayoutConfig, Side};
use adw::prelude::*;
use gtk4::{Align, Orientation};
use std::collections::HashMap;

// ═══════════════════════════════════════════════
//  Window Chrome: header bar + split panes
// ═══════════════════════════════════════════════
//
//   ┌ header: [start items]   [center items]   [end items] ┐
//   ├──────────┬────────────────────────────┬──────────────┤
//   │ sidebar  │          content           │  inspector   │
//   └──────────┴────────────────────────────┴──────────────┘
//
// Everything here is driven by `layout.toml` via `apply()`, which can run
// any number of times (hot reload). Buttons talk to the app only through
// `win.*` actions, so the chrome needs no access to `AppState`.

pub struct Chrome {
    window: adw::ApplicationWindow,
    pub toolbar: adw::ToolbarView,
    pub header: adw::HeaderBar,
    /// Outer split: places sidebar | (content + inspector).
    pub sidebar_split: adw::OverlaySplitView,
    /// Inner split: content | inspector.
    pub inspector_split: adw::OverlaySplitView,
    pub path_bar: gtk4::Box,
    pub new_button: gtk4::MenuButton,
    items: HashMap<HeaderItem, gtk4::Widget>,
    /// Fixed containers for the header's start / center / end items.
    /// Items only ever move between these (or get hidden): removing a
    /// widget from the window while its popover or tooltip is pending
    /// crashes GTK.
    slots: [gtk4::Box; 3],
    bp_inspector: adw::Breakpoint,
    bp_sidebar: adw::Breakpoint,
}

impl Chrome {
    /// Builds the chrome around `sidebar`, `content` and `inspector`.
    pub fn new(
        window: &adw::ApplicationWindow,
        sidebar: &impl IsA<gtk4::Widget>,
        content: &impl IsA<gtk4::Widget>,
        inspector: &impl IsA<gtk4::Widget>,
        menu: &gtk4::MenuButton,
    ) -> Chrome {
        let inspector_split = adw::OverlaySplitView::builder()
            .content(content)
            .sidebar(inspector)
            .sidebar_position(gtk4::PackType::End)
            .build();
        let sidebar_split = adw::OverlaySplitView::builder()
            .sidebar(sidebar)
            .content(&inspector_split)
            .build();

        let header = adw::HeaderBar::new();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&sidebar_split));
        window.set_content(Some(&toolbar));

        let path_bar = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(2)
            .css_classes(["path-bar"])
            .build();
        // Long paths scroll instead of forcing a wide window (tiling!).
        let path_scroll = gtk4::ScrolledWindow::builder()
            .child(&path_bar)
            .hscrollbar_policy(gtk4::PolicyType::External)
            .vscrollbar_policy(gtk4::PolicyType::Never)
            .propagate_natural_width(true)
            .hexpand(true)
            .min_content_width(80)
            .build();

        // folder-new rather than list-add: Tela's list-add-symbolic renders
        // blank under GTK 4.22 (seen on KDE; cause not yet pinned down).
        let new_button = gtk4::MenuButton::builder()
            .icon_name("folder-new-symbolic")
            .tooltip_text("New Folder or File")
            .build();

        let items = build_items(
            &sidebar_split,
            &inspector_split,
            &path_scroll,
            &new_button,
            menu,
        );

        let slot = || {
            gtk4::Box::builder()
                .spacing(6)
                .valign(Align::Center)
                .build()
        };
        let slots = [slot(), slot(), slot()];
        slots[1].set_hexpand(true);
        header.pack_start(&slots[0]);
        header.set_title_widget(Some(&slots[1]));
        header.pack_end(&slots[2]);
        // Every item lives in some slot from the start; `apply` arranges them.
        for widget in items.values() {
            widget.set_visible(false);
            slots[2].append(widget);
        }

        // Breakpoints: conditions are set from layout.toml in `apply()`.
        let placeholder = || adw::BreakpointCondition::parse("max-width: 1px").unwrap();
        let bp_inspector = adw::Breakpoint::new(placeholder());
        bp_inspector.add_setter(&inspector_split, "collapsed", Some(&true.to_value()));
        let bp_sidebar = adw::Breakpoint::new(placeholder());
        bp_sidebar.add_setter(&inspector_split, "collapsed", Some(&true.to_value()));
        bp_sidebar.add_setter(&sidebar_split, "collapsed", Some(&true.to_value()));
        // Quarter-screen tiles: the view switcher doesn't fit; the same
        // choices live in the main menu's View section.
        bp_sidebar.add_setter(
            &items[&HeaderItem::ViewSwitcher],
            "visible",
            Some(&false.to_value()),
        );
        window.add_breakpoint(bp_inspector.clone());
        window.add_breakpoint(bp_sidebar.clone());

        Chrome {
            window: window.clone(),
            toolbar,
            header,
            sidebar_split,
            inspector_split,
            path_bar,
            new_button,
            items,
            slots,
            bp_inspector,
            bp_sidebar,
        }
    }

    /// Applies `layout` to the window. Safe to call repeatedly.
    pub fn apply(&self, layout: &LayoutConfig) {
        // ── Decorations ──
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        let decorations = match layout.window.decorations {
            Decorations::Auto => auto_decorations(&desktop),
            other => other,
        };
        let buttons = decorations == Decorations::Full;
        self.header.set_show_start_title_buttons(buttons);
        self.header.set_show_end_title_buttons(buttons);
        self.toolbar
            .set_reveal_top_bars(decorations != Decorations::None);

        // ── Header items ──
        let groups = [
            &layout.header.start,
            &layout.header.center,
            &layout.header.end,
        ];
        for (slot, items) in self.slots.iter().zip(groups) {
            let mut previous: Option<gtk4::Widget> = None;
            for item in items {
                let widget = &self.items[item];
                if widget.parent().as_ref() != Some(slot.upcast_ref()) {
                    // Move between slots within the same tick: never parentless
                    // when GTK next processes events.
                    if let Some(parent) = widget.parent().and_downcast::<gtk4::Box>() {
                        parent.remove(widget);
                    }
                    slot.append(widget);
                }
                slot.reorder_child_after(widget, previous.as_ref());
                // The narrowest breakpoint hides the view switcher; don't undo that.
                let squeezed = *item == HeaderItem::ViewSwitcher
                    && self.window.current_breakpoint().as_ref() == Some(&self.bp_sidebar);
                widget.set_visible(!squeezed);
                previous = Some(widget.clone());
            }
        }
        let listed: Vec<&HeaderItem> = groups.iter().flat_map(|g| g.iter()).collect();
        for (item, widget) in &self.items {
            if !listed.contains(&item) {
                widget.set_visible(false);
            }
        }

        // ── Panes ──
        let s = &layout.sidebar;
        set_width(&self.sidebar_split, s.width);
        self.sidebar_split.set_show_sidebar(s.visible);

        let i = &layout.inspector;
        set_width(&self.inspector_split, i.width);
        self.inspector_split.set_show_sidebar(i.visible);
        self.inspector_split.set_sidebar_position(match i.position {
            Side::Left => gtk4::PackType::Start,
            Side::Right => gtk4::PackType::End,
        });

        let cond = |px: u32| {
            adw::BreakpointCondition::parse(&format!("max-width: {}px", px))
                .expect("static breakpoint syntax")
        };
        self.bp_inspector
            .set_condition(Some(&cond(layout.window.collapse_inspector_below)));
        self.bp_sidebar
            .set_condition(Some(&cond(layout.window.collapse_sidebar_below)));
    }
}

fn set_width(split: &adw::OverlaySplitView, width: u32) {
    let w = width as f64;
    split.set_min_sidebar_width(w);
    split.set_max_sidebar_width(w);
    // With min == max the fraction only matters when collapsed.
    split.set_sidebar_width_fraction(0.3);
}

fn build_items(
    sidebar_split: &adw::OverlaySplitView,
    inspector_split: &adw::OverlaySplitView,
    path: &gtk4::ScrolledWindow,
    new_button: &gtk4::MenuButton,
    menu: &gtk4::MenuButton,
) -> HashMap<HeaderItem, gtk4::Widget> {
    let action_button = |icon: &str, tooltip: &str, action: &str| -> gtk4::Widget {
        gtk4::Button::builder()
            .icon_name(icon)
            .tooltip_text(tooltip)
            .action_name(action)
            .build()
            .upcast()
    };
    let pane_toggle = |icon: &str, tooltip: &str, split: &adw::OverlaySplitView| -> gtk4::Widget {
        let btn = gtk4::ToggleButton::builder()
            .icon_name(icon)
            .tooltip_text(tooltip)
            .build();
        btn.bind_property("active", split, "show-sidebar")
            .bidirectional()
            .sync_create()
            .build();
        btn.upcast()
    };

    // Grid / List / Tree / Graph as a linked radio group on `win.view-mode`.
    let switcher = gtk4::Box::builder()
        .css_classes(["linked"])
        .valign(Align::Center)
        .build();
    for (mode, icon, tip) in [
        ("grid", "view-grid-symbolic", "Grid"),
        ("list", "view-list-symbolic", "List"),
        ("tree", "view-list-bullet-symbolic", "Tree"),
        ("graph", "network-workgroup-symbolic", "Graph"),
    ] {
        let b = gtk4::ToggleButton::builder()
            .icon_name(icon)
            .tooltip_text(tip)
            .action_name("win.view-mode")
            .action_target(&mode.to_variant())
            .build();
        switcher.append(&b);
    }

    use HeaderItem::*;
    HashMap::from([
        (
            SidebarToggle,
            pane_toggle(
                "sidebar-show-symbolic",
                "Toggle Sidebar (F9)",
                sidebar_split,
            ),
        ),
        (
            Back,
            action_button("go-previous-symbolic", "Back (Alt+←)", "win.back"),
        ),
        (
            Forward,
            action_button("go-next-symbolic", "Forward (Alt+→)", "win.forward"),
        ),
        (
            Up,
            action_button("go-up-symbolic", "Parent Folder (Alt+↑)", "win.go-up"),
        ),
        (Path, path.clone().upcast()),
        (New, new_button.clone().upcast()),
        (ViewSwitcher, switcher.upcast()),
        (
            InspectorToggle,
            pane_toggle(
                "sidebar-show-right-symbolic",
                "Toggle Inspector",
                inspector_split,
            ),
        ),
        (Menu, menu.clone().upcast()),
    ])
}
