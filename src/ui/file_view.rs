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
//   gio::ListStore<Entry> → FilterListModel → SortListModel → MultiSelection → GridView / ListView
//        ▲                  (search, Ctrl+F)   (folders first,   (click, Ctrl/Shift+click,
//        │                                      sort key,         rubber band, Ctrl+A)
//        │                                      group sections)
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
    /// Hides entries that don't match the search text.
    filter: gtk4::CustomFilter,
    filtered: gtk4::FilterListModel,
    /// Lowercased search text; empty shows everything.
    query: Rc<RefCell<String>>,
    pub search_bar: gtk4::SearchBar,
    search_entry: gtk4::SearchEntry,
    sorted: gtk4::SortListModel,
    selection: gtk4::MultiSelection,
    grid: gtk4::GridView,
    list: gtk4::ListView,
    scroll: gtk4::ScrolledWindow,
    /// Scrolled view + empty / error page on top.
    root: gtk4::Overlay,
    /// `root` above the status bar; what the window shows.
    pub widget: gtk4::Box,
    status: adw::StatusPage,
    /// Bottom bar: item counts, selection summary.
    counts: gtk4::Label,
    selection_summary: gtk4::Label,
    item_menu: gtk4::PopoverMenu,
    /// actions.toml entries that apply to the selection; filled on popup.
    custom_menu: gio::Menu,
    /// View-scope shortcuts from keybindings.toml (Delete, F2, …).
    shortcuts: gtk4::ShortcutController,
    state: RefCell<Weak<AppState>>,

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
        let query = Rc::new(RefCell::new(String::new()));
        let filter = {
            let query = query.clone();
            gtk4::CustomFilter::new(move |obj| {
                let query = query.borrow();
                query.is_empty() || entry_ref(obj).name.to_lowercase().contains(query.as_str())
            })
        };
        let filtered = gtk4::FilterListModel::new(Some(store.clone()), Some(filter.clone()));
        // The real sorter is set by `show` from the config.
        let sorted = gtk4::SortListModel::new(Some(filtered.clone()), None::<gtk4::Sorter>);
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
        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .hexpand(true)
            .child(&grid)
            .build();
        let status = adw::StatusPage::builder()
            .css_classes(["compact", "folder-status"])
            .can_target(false)
            .visible(false)
            .build();
        let root = gtk4::Overlay::builder()
            .child(&scroll)
            .vexpand(true)
            .build();
        root.add_overlay(&status);

        let counts = gtk4::Label::builder()
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        let selection_summary = gtk4::Label::builder()
            .xalign(1.0)
            .ellipsize(gtk4::pango::EllipsizeMode::Start)
            .build();
        let status_bar = gtk4::Box::builder()
            .spacing(12)
            .css_classes(["status-bar"])
            .build();
        status_bar.append(&counts);
        status_bar.append(&selection_summary);
        let search_entry = gtk4::SearchEntry::builder()
            .placeholder_text("Search this folder")
            .hexpand(true)
            .max_width_chars(40)
            .build();
        let search_bar = gtk4::SearchBar::builder()
            .child(&search_entry)
            .show_close_button(true)
            .build();
        search_bar.connect_entry(&search_entry);
        // Typing anywhere in the view starts a search.
        search_bar.set_key_capture_widget(Some(&root));

        let widget = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();
        widget.append(&search_bar);
        widget.append(&root);
        widget.append(&status_bar);
        // Key events bubble up from the focused grid / list to here.
        let shortcuts = gtk4::ShortcutController::new();
        root.add_controller(shortcuts.clone());

        let custom_menu = gio::Menu::new();
        let item_menu = gtk4::PopoverMenu::from_model(Some(&item_menu_model(&custom_menu)));
        item_menu.add_css_class("context-menu");
        item_menu.set_has_arrow(false);
        item_menu.set_halign(gtk4::Align::Start);
        item_menu.set_parent(&root);

        Rc::new(FileView {
            store,
            filter,
            filtered,
            query,
            search_bar,
            search_entry,
            sorted,
            selection,
            grid,
            list,
            scroll,
            root,
            widget,
            status,
            counts,
            selection_summary,
            item_menu,
            custom_menu,
            shortcuts,
            state: RefCell::new(Weak::new()),
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
        *self.state.borrow_mut() = Rc::downgrade(state);
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
                    view.update_selection_summary();
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

        // Files dropped on the background land in the current folder.
        attach_drop_target(&self.root, Rc::downgrade(state), None);

        {
            let view = Rc::downgrade(self);
            self.search_entry.connect_search_changed(move |entry| {
                if let Some(view) = view.upgrade() {
                    view.set_query(&entry.text());
                }
            });
        }
        {
            // Closing the bar (Escape, the close button) clears the search.
            let view = Rc::downgrade(self);
            self.search_bar
                .connect_search_mode_enabled_notify(move |bar| {
                    let Some(view) = view.upgrade() else { return };
                    if !bar.is_search_mode() {
                        view.search_entry.set_text("");
                        view.set_query("");
                        view.grab_focus();
                    }
                });
        }
        {
            // Enter in the search field opens the first match.
            let (view, state) = (Rc::downgrade(self), Rc::downgrade(state));
            self.search_entry.connect_activate(move |_| {
                let (Some(view), Some(state)) = (view.upgrade(), state.upgrade()) else {
                    return;
                };
                if let Some(entry) = entry_at(&view.sorted, 0) {
                    state.activate(&entry);
                }
            });
        }
    }

    // ─── Search ───

    /// Shows the search bar (Ctrl+F), or focuses it if already shown.
    pub fn start_search(&self) {
        self.search_bar.set_search_mode(true);
        self.search_entry.grab_focus();
    }

    fn set_query(&self, text: &str) {
        let text = text.trim().to_lowercase();
        let old = self.query.replace(text.clone());
        if old == text {
            return;
        }
        let change = if text.contains(old.as_str()) {
            gtk4::FilterChange::MoreStrict
        } else if old.contains(text.as_str()) {
            gtk4::FilterChange::LessStrict
        } else {
            gtk4::FilterChange::Different
        };
        self.filter.changed(change);
        self.update_status();
    }

    fn grab_focus(&self) {
        if let Some(view) = self.scroll.child() {
            view.grab_focus();
        }
    }

    // ─── Showing & loading ───

    /// Shows the folder in `state` with the configured mode, grouping and
    /// item style, and (re)lists it. The selection is kept by path.
    pub fn show(self: &Rc<Self>, state: &Rc<AppState>) {
        let cfg = state.config();

        let (by, descending) = (cfg.sort_by, cfg.sort_descending);
        self.sorted
            .set_sorter(Some(&gtk4::CustomSorter::new(move |a, b| {
                Entry::sort_cmp(&entry_ref(a), &entry_ref(b), by, descending).into()
            })));

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
        if self.dir.borrow().as_ref() != Some(&dir) {
            // A search is about one folder.
            self.search_bar.set_search_mode(false);
        }
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
        let error = self.error.borrow().clone();
        let empty = !self.loading.get() && self.store.n_items() == 0;
        let no_match = !self.loading.get() && !empty && self.filtered.n_items() == 0;
        if let Some(err) = &error {
            self.status
                .set_icon_name(Some("action-unavailable-symbolic"));
            self.status.set_title("Can’t Open This Folder");
            self.status
                .set_description(Some(&glib::markup_escape_text(err)));
        } else if empty {
            self.status.set_icon_name(Some("folder-symbolic"));
            self.status.set_title("This Folder Is Empty");
            self.status.set_description(Some(
                "Drop files here, or right-click to create a folder or a file.",
            ));
        } else if no_match {
            self.status.set_icon_name(Some("edit-find-symbolic"));
            self.status.set_title("No Results");
            self.status
                .set_description(Some("No item in this folder matches your search."));
        }
        self.status
            .set_visible(error.is_some() || empty || no_match);
        self.update_counts();
    }

    /// "4 folders, 12 files" on the left, the selection on the right.
    fn update_counts(&self) {
        if self.loading.get() {
            self.counts.set_label("Loading…");
            self.selection_summary.set_label("");
            return;
        }
        let n = self.store.n_items();
        let folders = (0..n)
            .filter(|&i| self.store.item(i).is_some_and(|o| entry_ref(&o).is_dir))
            .count();
        let mut counts = count_summary(folders, n as usize - folders);
        if !self.query.borrow().is_empty() {
            let shown = self.filtered.n_items();
            counts = format!(
                "{} match{} · {}",
                shown,
                if shown == 1 { "" } else { "es" },
                counts
            );
        }
        self.counts.set_label(&counts);
        self.update_selection_summary();
    }

    /// The right side of the status bar. Runs on every selection change,
    /// so it doesn't walk the whole folder like `update_counts`.
    fn update_selection_summary(&self) {
        if self.loading.get() {
            return;
        }
        let selected = self.selected_entries();
        let summary = match selected.as_slice() {
            [] => String::new(),
            [one] if one.is_dir => format!("“{}” selected", one.name),
            [one] => format!("“{}” selected ({})", one.name, one.size_display()),
            many => {
                let bytes: u64 = many.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
                if many.iter().all(|e| e.is_dir) {
                    format!("{} items selected", many.len())
                } else {
                    format!(
                        "{} items selected ({})",
                        many.len(),
                        crate::filesystem::format_size(bytes)
                    )
                }
            }
        };
        self.selection_summary.set_label(&summary);
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
        if let Some(state) = self.state.borrow().upgrade() {
            state.fill_action_menu(&self.custom_menu);
        }
        self.item_menu.popup();
    }

    /// Replaces the view-scope shortcuts: `(accelerator, "win.action")`.
    pub fn set_shortcuts(&self, shortcuts: &[(String, String)]) {
        while let Some(old) = self.shortcuts.item(0).and_downcast::<gtk4::Shortcut>() {
            self.shortcuts.remove_shortcut(&old);
        }
        for (trigger, action) in shortcuts {
            self.shortcuts.add_shortcut(gtk4::Shortcut::new(
                gtk4::ShortcutTrigger::parse_string(trigger),
                Some(gtk4::NamedAction::new(action)),
            ));
        }
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

    // Folders accept drops: the files move (or copy) into them.
    if entry.is_dir {
        if let Some(state) = view.upgrade().map(|v| v.state.borrow().clone()) {
            attach_drop_target(widget, state, Some(entry.path.clone()));
        }
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

/// Accepts dropped files into `dest`, or the current folder when `None`.
/// Ctrl copies, Shift moves; otherwise same disk moves, another copies.
fn attach_drop_target(
    widget: &impl IsA<gtk4::Widget>,
    state: Weak<AppState>,
    dest: Option<PathBuf>,
) {
    let target = gtk4::DropTarget::new(
        gdk::FileList::static_type(),
        gdk::DragAction::COPY | gdk::DragAction::MOVE,
    );
    target.connect_drop(move |target, value, _, _| {
        let (Some(state), Ok(files)) = (state.upgrade(), value.get::<gdk::FileList>()) else {
            return false;
        };
        let paths: Vec<PathBuf> = files.files().iter().filter_map(|f| f.path()).collect();
        let dest = dest.clone().unwrap_or_else(|| state.current_path());
        state.drop_files(paths, dest, target.current_event_state());
        true
    });
    widget.add_controller(target);
}

/// "3 folders, 1 file", "Empty".
fn count_summary(folders: usize, files: usize) -> String {
    let plural = |n: usize, word: &str| format!("{} {}{}", n, word, if n == 1 { "" } else { "s" });
    match (folders, files) {
        (0, 0) => "Empty".into(),
        (f, 0) => plural(f, "folder"),
        (0, n) => plural(n, "file"),
        (f, n) => format!("{}, {}", plural(f, "folder"), plural(n, "file")),
    }
}

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
/// `custom` holds the applicable actions.toml entries.
fn item_menu_model(custom: &gio::Menu) -> gio::Menu {
    let menu = gio::Menu::new();
    let open = gio::Menu::new();
    open.append(Some("Open"), Some("win.open-selection"));
    open.append(Some("Rename…"), Some("win.rename-selection"));
    open.append(Some("Add to Bookmarks"), Some("win.bookmark"));
    menu.append_section(None, &open);
    let clipboard = gio::Menu::new();
    clipboard.append(Some("Cut"), Some("win.cut"));
    clipboard.append(Some("Copy"), Some("win.copy"));
    menu.append_section(None, &clipboard);
    menu.append_section(None, custom);
    let remove = gio::Menu::new();
    remove.append(Some("Move to Trash"), Some("win.trash-selection"));
    remove.append(Some("Delete Permanently…"), Some("win.delete-selection"));
    menu.append_section(None, &remove);
    menu
}

#[cfg(test)]
mod tests {
    use super::count_summary;

    #[test]
    fn count_summaries() {
        assert_eq!(count_summary(0, 0), "Empty");
        assert_eq!(count_summary(1, 0), "1 folder");
        assert_eq!(count_summary(0, 2), "2 files");
        assert_eq!(count_summary(3, 1), "3 folders, 1 file");
    }
}
