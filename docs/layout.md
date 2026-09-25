# Layout

`~/.config/diptych/layout.toml` controls the window chrome and panes. It is created on first launch with
every option documented. Changes apply **as soon as you save**. If the file has a mistake, the previous
layout stays and the error is printed. Every key is optional.

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
