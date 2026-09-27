use crate::config::{AppConfig, GroupBy, OpenWith, ViewMode};
use crate::filesystem::Entry;
use crate::ui::state::AppState;
use crate::ui::{context_menu, drag_source, preview, widgets};
use gtk4::prelude::*;
use gtk4::{gdk, glib::BoxedAnyObject, graphene};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

// ═══════════════════════════════════════════════
//  File View — virtualized grid / list of the current folder
// ═══════════════════════════════════════════════
//
//   gio::ListStore<Entry>  →  SortListModel  →  MultiSelection  →  GridView / ListView
//        ▲                     (folders first,     (click, Ctrl/Shift+click,
//        │                      group sections)     rubber band, Ctrl+A)
//   async enumeration in batches + gio::FileMonitor live updates
//
// Only the visible items have widgets: they are built on `bind` and
// dropped on `unbind`, so a folder with 50 000 files costs a few dozen
// widgets. The folder is listed with GIO's async enumerator, so the UI
// never blocks on a slow disk; a newer `load` makes older ones bail out.
//
// The selection lives in the `MultiSelection`; every change is mirrored
// into `AppState` (`set_selection`), which drives the inspector and the
// `win.*-selection` actions.

/// Entries enumerated per async batch.
const BATCH: i32 = 256;
/// Minimum time between inserting buffered entries into the model.
const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
const ATTRIBUTES: &str = "standard::name,standard::type,standard::size,time::modified";

pub struct FileView {
    store: gio::ListStore,
    sorted: gtk4::SortListModel,
    selection: gtk4::MultiSelection,
    grid: gtk4::GridView,
    list: gtk4::ListView,
    scroll: gtk4::ScrolledWindow,
    /// Scrolled view + empty / error message on top.
    pub root: gtk4::Overlay,
    status: gtk4::Label,
    item_menu: gtk4::PopoverMenu,

    dir: RefCell<Option<PathBuf>>,
    show_hidden: Cell<bool>,
    /// Paths in `store`, for O(1) duplicate checks while events race the listing.
    paths: RefCell<HashSet<PathBuf>>,
    /// Bumped by every `load` / `stop`; stale async work compares and bails.
    generation: Cell<u64>,
    loading: Cell<bool>,
    error: RefCell<Option<String>>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    /// Paths to select once the listing finishes (kept across reloads).
    pending: RefCell<Vec<PathBuf>>,
    /// While true, model churn isn't mirrored into `AppState`.
    sync_paused: Cell<bool>,
    /// Widget to anchor popovers (rename) at: the last right-clicked item.
    anchor: RefCell<glib::WeakRef<gtk4::Widget>>,
    inspector_refresh_queued: Cell<bool>,
}

impl FileView {
    pub fn new() -> Rc<FileView> {
        let store = gio::ListStore::new::<BoxedAnyObject>();
        let sorter =
            gtk4::CustomSorter::new(|a, b| Entry::display_cmp(&entry_ref(a), &entry_ref(b)).into());
        let sorted = gtk4::SortListModel::new(Some(store.clone()), Some(sorter));
        let selection = gtk4::MultiSelection::new(Some(sorted.clone()));

        let grid = gtk4::GridView::builder()
            .model(&selection)
            .max_columns(64)
            .enable_rubberband(true)
            .css_classes(["file-grid"])
            .build();
        let list = gtk4::ListView::builder()
            .model(&selection)
            .enable_rubberband(true)
            .css_classes(["file-list"])
            .build();
        for view in [grid.upcast_ref::<gtk4::Widget>(), list.upcast_ref()] {
            view.add_controller(selection_shortcuts());
        }

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .hexpand(true)
            .child(&grid)
            .build();
        let status = gtk4::Label::builder()
            .css_classes(["inspector-subtitle"])
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .justify(gtk4::Justification::Center)
            .wrap(true)
            .can_target(false)
            .visible(false)
            .build();
        let root = gtk4::Overlay::builder().child(&scroll).build();
        root.add_overlay(&status);

        let item_menu = gtk4::PopoverMenu::from_model(Some(&item_menu_model()));
        item_menu.add_css_class("context-menu");
        item_menu.set_has_arrow(false);
        item_menu.set_halign(gtk4::Align::Start);
        item_menu.set_parent(&root);

        Rc::new(FileView {
            store,
            sorted,
            selection,
            grid,
            list,
            scroll,
            root,
            status,
            item_menu,
            dir: RefCell::new(None),
            show_hidden: Cell::new(false),
            paths: RefCell::new(HashSet::new()),
            generation: Cell::new(0),
            loading: Cell::new(false),
            error: RefCell::new(None),
            monitor: RefCell::new(None),
            pending: RefCell::new(vec![]),
            sync_paused: Cell::new(false),
            anchor: RefCell::new(glib::WeakRef::new()),
            inspector_refresh_queued: Cell::new(false),
        })
    }

    /// Connects the view to the app. Call once, after `AppState` exists.
    pub fn bind(self: &Rc<Self>, state: &Rc<AppState>) {
        for view in [
            self.grid.upcast_ref::<gtk4::Widget>(),
            self.list.upcast_ref(),
        ] {
            let (weak_view, weak_state) = (Rc::downgrade(self), Rc::downgrade(state));
            let on_activate = move |pos: u32| {
                let (Some(view), Some(state)) = (weak_view.upgrade(), weak_state.upgrade()) else {
                    return;
                };
                if let Some(entry) = entry_at(&view.sorted, pos) {
                    state.activate(&entry);
                }
            };
            if let Some(grid) = view.downcast_ref::<gtk4::GridView>() {
                grid.connect_activate(move |_, pos| on_activate(pos));
            } else if let Some(list) = view.downcast_ref::<gtk4::ListView>() {
                list.connect_activate(move |_, pos| on_activate(pos));
            }
        }

        {
            let (view, state) = (Rc::downgrade(self), Rc::downgrade(state));
            self.selection.connect_selection_changed(move |_, _, _| {
                if let (Some(view), Some(state)) = (view.upgrade(), state.upgrade()) {
                    view.sync(&state);
                }
            });
        }
        {
            // Removed items leave the selection without a selection-changed.
            let (view, state) = (Rc::downgrade(self), Rc::downgrade(state));
            self.selection.connect_items_changed(move |_, _, _, _| {
                if let (Some(view), Some(state)) = (view.upgrade(), state.upgrade()) {
                    view.sync(&state);
                    view.update_status();
                }
            });
        }

        // Left click on empty space clears the selection.
        {
            let gesture = gtk4::GestureClick::builder().button(1).build();
            let view = Rc::downgrade(self);
            gesture.connect_pressed(move |_, _, x, y| {
                let Some(view) = view.upgrade() else { return };
                let picked = view.root.pick(x, y, gtk4::PickFlags::DEFAULT);
                let on_item = std::iter::successors(picked, |w| w.parent()).any(|w| {
                    w.has_css_class("file-card")
                        || w.has_css_class("file-row")
                        || w.is::<gtk4::Scrollbar>()
                });
                if !on_item {
                    view.selection.unselect_all();
                }
            });
            self.root.add_controller(gesture);
        }

        context_menu::attach_background_context_menu(&self.root, state);
    }

    // ─── Showing & loading ───

    /// Shows the folder in `state` with the configured mode, grouping and
    /// item style, and (re)lists it. The selection is kept by path.
    pub fn show(self: &Rc<Self>, state: &Rc<AppState>) {
        let cfg = state.config();

        match cfg.grouping {
            GroupBy::None => {
                self.sorted.set_section_sorter(None::<&gtk4::Sorter>);
                self.list.set_header_factory(None::<&gtk4::ListItemFactory>);
            }
            ref grouping => {
                let g = grouping.clone();
                let section_sorter = gtk4::CustomSorter::new(move |a, b| {
                    entry_ref(a)
                        .group_key(&g)
                        .cmp(&entry_ref(b).group_key(&g))
                        .into()
                });
                self.sorted.set_section_sorter(Some(&section_sorter));
                self.list
                    .set_header_factory(Some(&header_factory(grouping.clone())));
            }
        }

        let factory = self.item_factory(&cfg);
        let single_click = cfg.open_with == OpenWith::SingleClick;
        if cfg.view_mode == ViewMode::List {
            self.list.set_factory(Some(&factory));
            self.list.set_single_click_activate(single_click);
            self.scroll.set_child(Some(&self.list));
        } else {
            self.grid.set_factory(Some(&factory));
            self.grid.set_single_click_activate(single_click);
            self.scroll.set_child(Some(&self.grid));
        }

        self.load(state, state.current_path(), cfg.show_hidden);
    }

    fn load(self: &Rc<Self>, state: &Rc<AppState>, dir: PathBuf, show_hidden: bool) {
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        *self.pending.borrow_mut() = state.selection();
        self.loading.set(true);
        *self.error.borrow_mut() = None;
        self.store.remove_all();
        self.paths.borrow_mut().clear();
        *self.dir.borrow_mut() = Some(dir.clone());
        self.show_hidden.set(show_hidden);
        self.watch(state, &dir);
        self.update_status();

        let (view, weak_state) = (self.clone(), Rc::downgrade(state));
        glib::spawn_future_local(async move {
            let result = view.enumerate(generation, &dir).await;
            if view.generation.get() != generation {
                return;
            }
            if let Err(e) = result {
                eprintln!("Failed to list {}: {}", dir.display(), e);
                *view.error.borrow_mut() = Some(e.message().to_string());
            }
            view.loading.set(false);
            let pending = std::mem::take(&mut *view.pending.borrow_mut());
            view.scroll_to(0, gtk4::ListScrollFlags::NONE);
            view.select_paths(&pending, true);
            if let Some(state) = weak_state.upgrade() {
                view.sync(&state);
            }
            view.update_status();
        });
    }

    async fn enumerate(&self, generation: u64, dir: &Path) -> Result<(), glib::Error> {
        let enumerator = gio::File::for_path(dir)
            .enumerate_children_future(
                ATTRIBUTES,
                gio::FileQueryInfoFlags::NONE,
                glib::Priority::DEFAULT,
            )
            .await?;
        // Every store insert makes the sorted model re-sort, so batches are
        // buffered and flushed at most every FLUSH_INTERVAL: the first
        // screenful shows up at once, a huge folder costs a handful of sorts.
        let mut buffer: Vec<BoxedAnyObject> = vec![];
        let mut last_flush: Option<std::time::Instant> = None;
        loop {
            let infos = enumerator
                .next_files_future(BATCH, glib::Priority::DEFAULT)
                .await?;
            if self.generation.get() != generation {
                return Ok(());
            }
            let done = infos.is_empty();
            let show_hidden = self.show_hidden.get();
            let mut paths = self.paths.borrow_mut();
            buffer.extend(
                infos
                    .iter()
                    .map(|info| Entry::from_file_info(dir, info))
                    .filter(|e| show_hidden || !e.is_hidden())
                    .filter(|e| paths.insert(e.path.clone()))
                    .map(BoxedAnyObject::new),
            );
            drop(paths);
            if done || last_flush.is_none_or(|t| t.elapsed() >= FLUSH_INTERVAL) {
                self.store.extend_from_slice(&std::mem::take(&mut buffer));
                last_flush = Some(std::time::Instant::now());
                // Items land all over the sorted order; stay at the top
                // instead of following whichever item the view anchored to.
                self.scroll_to(0, gtk4::ListScrollFlags::NONE);
            }
            if done {
                return Ok(());
            }
        }
    }

    /// Drops the listing and the watch (another view mode took over).
    pub fn stop(&self) {
        self.generation.set(self.generation.get() + 1);
        if let Some(monitor) = self.monitor.take() {
            monitor.cancel();
        }
        *self.dir.borrow_mut() = None;
        self.loading.set(false);
        self.sync_paused.set(true);
        self.store.remove_all();
        self.sync_paused.set(false);
        self.paths.borrow_mut().clear();
    }

    /// Whether the folder is watched, i.e. changes on disk show up by themselves.
    pub fn is_live(&self) -> bool {
        self.monitor.borrow().is_some()
    }

    fn update_status(&self) {
        let message = if let Some(err) = self.error.borrow().as_ref() {
            Some(format!("Can’t show this folder\n{}", err))
        } else if !self.loading.get() && self.store.n_items() == 0 {
            Some("This folder is empty".to_string())
        } else {
            None
        };
        self.status.set_visible(message.is_some());
        if let Some(message) = message {
            self.status.set_label(&message);
        }
    }

    // ─── Live updates ───

    fn watch(self: &Rc<Self>, state: &Rc<AppState>, dir: &Path) {
        if let Some(old) = self.monitor.take() {
            old.cancel();
        }
        let monitor = match gio::File::for_path(dir)
            .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[watch] {}: {} (no live updates)", dir.display(), e);
                return;
            }
        };
        let (view, state) = (Rc::downgrade(self), Rc::downgrade(state));
        monitor.connect_changed(move |_, file, other, event| {
            if let (Some(view), Some(state)) = (view.upgrade(), state.upgrade()) {
                view.on_disk_change(&state, file, other, event);
            }
        });
        *self.monitor.borrow_mut() = Some(monitor);
    }

    fn on_disk_change(
        self: &Rc<Self>,
        state: &Rc<AppState>,
        file: &gio::File,
        other: Option<&gio::File>,
        event: gio::FileMonitorEvent,
    ) {
        use gio::FileMonitorEvent as E;
        let (Some(path), Some(dir)) = (file.path(), self.dir.borrow().clone()) else {
            return;
        };

        if path == dir {
            // The folder itself was deleted or moved: fall back to what's left.
            if matches!(event, E::Deleted | E::MovedOut | E::Renamed) {
                let state = state.clone();
                glib::idle_add_local_once(move || {
                    if let Some(existing) = dir.ancestors().find(|p| p.is_dir()) {
                        state.navigate_to(existing.to_path_buf());
                    }
                });
            }
            return;
        }

        // Replacing an item drops its selection; remember and restore it.
        let mut keep = state.selection();
        self.sync_paused.set(true);
        match event {
            E::Created | E::MovedIn | E::Changed | E::ChangesDoneHint | E::AttributeChanged => {
                self.upsert(&path)
            }
            E::Deleted | E::MovedOut => self.remove(&path),
            E::Renamed => {
                self.remove(&path);
                if let Some(new_path) = other.and_then(|o| o.path()) {
                    if keep.contains(&path) {
                        keep.push(new_path.clone());
                    }
                    self.upsert(&new_path);
                }
            }
            _ => {}
        }
        self.sync_paused.set(false);
        self.select_paths(&keep, false);
        self.sync(state);
        self.update_status();
        self.schedule_inspector_refresh(state);
    }

    /// The inspector summarizes the folder when nothing is selected; keep
    /// its item count current without recounting on every event of a burst.
    fn schedule_inspector_refresh(&self, state: &Rc<AppState>) {
        if !state.selection().is_empty() || self.inspector_refresh_queued.replace(true) {
            return;
        }
        let state = Rc::downgrade(state);
        glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
            if let Some(state) = state.upgrade() {
                state.file_view.inspector_refresh_queued.set(false);
                crate::ui::inspector::refresh(&state);
            }
        });
    }

    /// Adds `path` to the store, or refreshes its entry if already there.
    fn upsert(&self, path: &Path) {
        if std::fs::symlink_metadata(path).is_err() {
            return; // already gone again
        }
        let entry = Entry::from_path(path);
        if !self.show_hidden.get() && entry.is_hidden() {
            return;
        }
        let item = BoxedAnyObject::new(entry);
        if self.paths.borrow().contains(path) {
            if let Some(pos) = self.store_position(path) {
                self.store.splice(pos, 1, &[item]);
            }
        } else {
            self.paths.borrow_mut().insert(path.to_path_buf());
            self.store.append(&item);
        }
    }

    fn remove(&self, path: &Path) {
        if self.paths.borrow_mut().remove(path) {
            if let Some(pos) = self.store_position(path) {
                self.store.remove(pos);
            }
        }
    }

    fn store_position(&self, path: &Path) -> Option<u32> {
        (0..self.store.n_items()).find(|&i| {
            self.store
                .item(i)
                .is_some_and(|o| entry_ref(&o).path == path)
        })
    }

    // ─── Selection ───

    /// Mirrors the model's selection into `AppState`.
    fn sync(&self, state: &Rc<AppState>) {
        if self.loading.get() || self.sync_paused.get() {
            return;
        }
        let paths: Vec<PathBuf> = self
            .selected_entries()
            .into_iter()
            .map(|e| e.path)
            .collect();
        if paths != state.selection() {
            // A new selection: stop anchoring popovers at the old item.
            self.anchor.borrow().set(None);
        }
        state.set_selection(paths);
    }

    pub fn selected_entries(&self) -> Vec<Entry> {
        let set = self.selection.selection();
        (0..set.size())
            .filter_map(|i| entry_at(&self.sorted, set.nth(i as u32)))
            .collect()
    }

    /// Adds `paths` (those present) to the selection; optionally scrolls to
    /// and focuses the first one.
    fn select_paths(&self, paths: &[PathBuf], scroll: bool) {
        if paths.is_empty() {
            return;
        }
        let wanted: HashSet<&PathBuf> = paths.iter().collect();
        let mut first = None;
        self.sync_paused.set(true);
        for pos in 0..self.sorted.n_items() {
            let hit = self
                .sorted
                .item(pos)
                .is_some_and(|o| wanted.contains(&entry_ref(&o).path));
            if hit && !self.selection.is_selected(pos) {
                self.selection.select_item(pos, false);
                first.get_or_insert(pos);
            }
        }
        self.sync_paused.set(false);
        if let (true, Some(pos)) = (scroll, first) {
            self.scroll_to(pos, gtk4::ListScrollFlags::FOCUS);
        }
    }

    fn scroll_to(&self, pos: u32, flags: gtk4::ListScrollFlags) {
        if pos >= self.sorted.n_items() {
            return;
        }
        if self.scroll.child().as_ref() == Some(self.list.upcast_ref()) {
            self.list.scroll_to(pos, flags, None);
        } else {
            self.grid.scroll_to(pos, flags, None);
        }
    }

    pub fn select_all(&self) {
        self.selection.select_all();
    }

    pub fn unselect_all(&self) {
        self.selection.unselect_all();
    }

    /// Where to anchor a popover about the selection (e.g. rename).
    pub fn anchor(&self) -> gtk4::Widget {
        let mapped = |w: &gtk4::Widget| w.is_mapped();
        if let Some(w) = self.anchor.borrow().upgrade().filter(mapped) {
            return w;
        }
        let view: gtk4::Widget = self.scroll.child().unwrap_or(self.root.clone().upcast());
        view.focus_child()
            .filter(mapped)
            .unwrap_or_else(|| self.root.clone().upcast())
    }

    fn popup_item_menu(&self, item: &gtk4::Widget, x: f64, y: f64) {
        let point = item
            .compute_point(&self.root, &graphene::Point::new(x as f32, y as f32))
            .unwrap_or_else(|| graphene::Point::new(0.0, 0.0));
        self.item_menu.set_pointing_to(Some(&gdk::Rectangle::new(
            point.x() as i32,
            point.y() as i32,
            1,
            1,
        )));
        self.item_menu.popup();
    }

    // ─── Item widgets ───

    fn item_factory(self: &Rc<Self>, cfg: &AppConfig) -> gtk4::SignalListItemFactory {
        let factory = gtk4::SignalListItemFactory::new();
        let cfg = cfg.clone();
        let weak_view = Rc::downgrade(self);
        factory.connect_bind(move |_, obj| {
            let Some(item) = obj.downcast_ref::<gtk4::ListItem>() else {
                return;
            };
            let Some(entry) = item.item().map(|o| entry_ref(&o).clone()) else {
                return;
            };
            let widget = if cfg.view_mode == ViewMode::List {
                widgets::create_file_row(&entry, &cfg)
            } else {
                widgets::create_file_card(&entry, &cfg)
            };
            decorate_item(&widget, &entry, item, &weak_view);
            item.set_child(Some(&widget));
        });
        factory.connect_unbind(|_, obj| {
            if let Some(item) = obj.downcast_ref::<gtk4::ListItem>() {
                item.set_child(None::<&gtk4::Widget>);
            }
        });
        factory
    }
}

/// Per-item behavior: tooltip preview, right-click menu, drag source.
fn decorate_item(widget: &gtk4::Box, entry: &Entry, item: &gtk4::ListItem, view: &Weak<FileView>) {
    // Hover tooltip with image preview for supported formats
    if preview::supports_preview(&entry.path) {
        let path = entry.path.clone();
        widget.set_has_tooltip(true);
        widget.connect_query_tooltip(
            move |_, _, _, _, tooltip| match preview::build_tooltip_preview(&path) {
                Some(img) => {
                    tooltip.set_custom(Some(&img));
                    true
                }
                None => false,
            },
        );
    } else {
        widget.set_tooltip_text(Some(&entry.name));
    }

    // Right click: select the item (unless it's part of the selection),
    // then show the shared item menu.
    {
        let gesture = gtk4::GestureClick::builder().button(3).build();
        let (item, view) = (item.downgrade(), view.clone());
        gesture.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            let (Some(item), Some(view)) = (item.upgrade(), view.upgrade()) else {
                return;
            };
            let pos = item.position();
            if !view.selection.is_selected(pos) {
                view.selection.select_item(pos, true);
            }
            let Some(widget) = gesture.widget() else {
                return;
            };
            view.anchor.borrow().set(Some(&widget));
            view.popup_item_menu(&widget, x, y);
        });
        widget.add_controller(gesture);
    }

    // Dragging a selected item drags the whole selection.
    {
        let (item, view, single) = (item.downgrade(), view.clone(), entry.clone());
        drag_source::attach_drag_source(widget, move || {
            let (Some(item), Some(view)) = (item.upgrade(), view.upgrade()) else {
                return vec![];
            };
            let entries = if view.selection.is_selected(item.position()) {
                view.selected_entries()
            } else {
                vec![single.clone()]
            };
            entries.into_iter().map(|e| (e.path, e.is_dir)).collect()
        });
    }
}

// ─── Helpers ───

/// Borrows the `Entry` inside a model item.
fn entry_ref(obj: &impl IsA<glib::Object>) -> std::cell::Ref<'_, Entry> {
    obj.as_ref()
        .downcast_ref::<BoxedAnyObject>()
        .expect("file model items are BoxedAnyObject<Entry>")
        .borrow::<Entry>()
}

fn entry_at(model: &impl IsA<gio::ListModel>, pos: u32) -> Option<Entry> {
    model.item(pos).map(|o| entry_ref(&o).clone())
}

/// Section headers for the list view when grouping is on.
fn header_factory(grouping: GroupBy) -> gtk4::SignalListItemFactory {
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_setup(|_, obj| {
        if let Some(header) = obj.downcast_ref::<gtk4::ListHeader>() {
            let label = gtk4::Label::builder()
                .css_classes(["group-header"])
                .halign(gtk4::Align::Start)
                .build();
            header.set_child(Some(&label));
        }
    });
    factory.connect_bind(move |_, obj| {
        let Some(header) = obj.downcast_ref::<gtk4::ListHeader>() else {
            return;
        };
        let (Some(item), Some(label)) =
            (header.item(), header.child().and_downcast::<gtk4::Label>())
        else {
            return;
        };
        label.set_label(&entry_ref(&item).group_key(&grouping));
    });
    factory
}

/// The item context menu; every entry is a `win.*` selection action.
fn item_menu_model() -> gio::Menu {
    let menu = gio::Menu::new();
    let open = gio::Menu::new();
    open.append(Some("Open"), Some("win.open-selection"));
    open.append(Some("Rename…"), Some("win.rename-selection"));
    menu.append_section(None, &open);
    let remove = gio::Menu::new();
    remove.append(Some("Move to Trash"), Some("win.trash-selection"));
    remove.append(Some("Delete Permanently…"), Some("win.delete-selection"));
    menu.append_section(None, &remove);
    menu
}

/// Keys that act on the selection while the view has focus.
fn selection_shortcuts() -> gtk4::ShortcutController {
    let controller = gtk4::ShortcutController::new();
    for (trigger, action) in [
        ("Escape", "win.unselect-all"),
        ("Delete", "win.trash-selection"),
        ("<Shift>Delete", "win.delete-selection"),
        ("F2", "win.rename-selection"),
    ] {
        controller.add_shortcut(gtk4::Shortcut::new(
            gtk4::ShortcutTrigger::parse_string(trigger),
            Some(gtk4::NamedAction::new(action)),
        ));
    }
    controller
}
