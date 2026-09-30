# Layout

`~/.config/diptych/layout.toml` controls the window chrome and panes. It is created on first launch with
every option documented. Changes apply **as soon as you save**. If the file has a mistake, the previous
layout stays and a message at the bottom of the window says what's wrong. Every key is optional.

**Customize → Layout** (`Ctrl+,`) edits the same file: panes, widths, inspector side and fields,
title bar style, and which header bar items are shown. Item *order* is only in the file.

```
┌ header: [start]           [center]            [end] ┐
├──────────┬──────────────────────────┬───────────────┤
│ sidebar  │          files           │   inspector   │
└──────────┴──────────────────────────┴───────────────┘
```

## `[window]`

| Key | Default | Meaning |
|---|---|---|
| `decorations` | `"auto"` | `auto`, `full`, `minimal` or `none`. See [Desktops](desktops.md). |
| `collapse-inspector-below` | `900` | Window width in px below which the inspector becomes an overlay (toggle it from the header). |
| `collapse-sidebar-below` | `600` | Below this width the sidebar collapses too, and the view switcher moves into the main menu. |

## `[sidebar]`

| Key | Default | Meaning |
|---|---|---|
| `visible` | `true` | Shown at startup (the header toggle still works) |
| `width` | `220` | px, 120–600 |

## `[inspector]`

| Key | Default | Meaning |
|---|---|---|
| `visible` | `true` | Shown at startup |
| `width` | `300` | px, 180–800 |
| `position` | `"right"` | `left` or `right` |
| `fields` | `["kind", "size", "modified", "dimensions", "location", "permissions"]` | Any of `kind`, `size`, `modified`, `created`, `dimensions`, `location`, `permissions`, in the order you want them. |

## `[header]`

Three lists of items, in display order:

| Item | What it is |
|---|---|
| `sidebar-toggle` | Show or hide the sidebar |
| `back`, `forward`, `up` | Navigation |
| `path` | Clickable path segments (`Home / projects / Diptych`) |
| `new` | New folder / file popover |
| `view-switcher` | Grid · List · Tree · Graph |
| `inspector-toggle` | Show or hide the inspector |
| `menu` | Main menu. Keep it somewhere, or use F10. |

An item can appear at most once. Leave an item out to hide it.

```toml
# A minimal, keyboard-first header
[header]
start = ["back"]
center = ["path"]
end = ["menu"]
```

## Behavior

`open_with` in `config.toml` (also under Settings → Behavior) chooses between `DoubleClick`, where a
click selects an item and a double click opens it, and `SingleClick`, where a click opens it.

## Sorting, search and bookmarks

The main menu's **Sort By** submenu sorts the grid and list by name, size, modified date or type,
ascending or descending. Folders always come first. The choice is saved in `config.toml`
(`sort_by`, `sort_descending`).

`Ctrl+F`, or just typing while the file view has focus, filters the current folder by name.
`Enter` opens the first match and `Escape` closes the search.

Bookmarks live in the GTK bookmarks file (`~/.config/gtk-3.0/bookmarks`), so they are shared with
Nautilus and the GTK file chooser. Add one with `Ctrl+D`, the item menu or the background menu;
right-click a bookmark in the sidebar to remove it.

## Copy, move and drag and drop

`Ctrl+C` / `Ctrl+X` put the selection on the clipboard as `x-special/gnome-copied-files`, a file
list (`text/uri-list`) and plain paths, so files copied in Diptych paste in Nautilus, Dolphin or a
terminal, and the other way round. `Ctrl+V` pastes into the current folder. Nothing is ever
overwritten: a name clash becomes `notes (2).txt`.

Files dropped on the background land in the current folder; dropped on a folder, they go into it.
Within one disk a drop moves, onto another disk it copies. Hold `Ctrl` to copy or `Shift` to move.
Transfers run in the background; a toast appears if one takes a while, and failures are reported.
