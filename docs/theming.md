# Theming Diptych

Diptych's look is built from **tokens**. Colors, corner radius, spacing density and fonts come from a
theme file, and the app generates its stylesheet from them. Edit the files below and save: changes
apply immediately, without a restart.

| File (in `~/.config/diptych/`) | Purpose |
|---|---|
| `theme.toml` | Picks a preset (`base`) and overrides any of its tokens. Created on first launch. |
| `themes/<id>.toml` | Your own presets. Use them with `base = "<id>"`, and they appear in Settings → Theme. |
| `user.css` | Plain GTK CSS, loaded last. Overrides anything. |

If `theme.toml` has a mistake (a typo, a bad color), Diptych keeps the previous theme and prints the
error (for example ``unknown color token “acent”``). It also shows the error under Settings → Theme.

## Resolution order

```
catppuccin-mocha (fills any gaps)  ←  base preset (and its own base, if it has one)  ←  theme.toml
```

So a `theme.toml` can be as short as one line:

```toml
base = "nord"
```

## Built-in presets

`catppuccin-mocha` (default), `rose-pine`, `tokyo-soft`, `nord`, `gruvbox`, `cozy-latte` (light),
`deep-dark`, `high-contrast`.

Their sources live in [`src/theme/presets/`](../src/theme/presets). Copy one to
`~/.config/diptych/themes/mine.toml` as a starting point for your own theme.

## Tokens

### Top level

| Key | Type | Meaning |
|---|---|---|
| `base` | string | Preset id to start from. Letters, digits, `-` and `_` only. |
| `name` | string | Display name in Settings. |
| `dark` | bool | Tells libadwaita whether to draw its stock widgets dark or light. |

### `[colors]`

Colors accept `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)`, `rgba(r, g, b, a)`, `transparent`, or a
reference to another token such as `"@window"`.

| Token | Used for |
|---|---|
| `window` | Main background |
| `sidebar` | Sidebar, header bar |
| `surface` | Cards, entries, menus, badges |
| `hover` | Hover / pressed backgrounds |
| `text` | Primary text |
| `text-secondary` | Section titles, secondary labels |
| `text-muted` | Metadata (sizes, dates), hints |
| `text-subtle` | Values in settings and inspector |
| `accent` | Highlights, selection, primary buttons, switches |
| `accent-hover` | Primary button hover |
| `accent-text` | Text on top of `accent` (usually `@window`) |
| `border`, `border-hover` | Hairlines and hover outlines |
| `shadow`, `shadow-hover` | Card and popover shadows |
| `danger` | Destructive actions ("Delete Permanently…") |

These also drive libadwaita's own colors (`--accent-bg-color`, `--window-bg-color`, and so on), so
stock dialogs and switches match.

### `[files]`

These are the colors for each file kind, used by the tree view names and by colored icons. Inside
`[files]`, `"@accent"` refers to a color token and `"@file-rust"` refers to another file color.

`folder`, `rust`, `python`, `js`, `c`, `java`, `go`, `script`, `image`, `audio`, `video`, `archive`,
`pdf`, `web`, `text`, `config`, `default`

### `[shape]`

| Key | Default | Meaning |
|---|---|---|
| `radius` | `14` | Base corner radius in px (0–64). Cards use it as-is and smaller elements use a fraction of it. |
| `density` | `"comfortable"` | `compact` (×0.75), `comfortable` (×1), `spacious` (×1.25) applied to paddings and margins. |

### `[fonts]`

| Key | Default | Meaning |
|---|---|---|
| `ui` | `"Inter, Cantarell, sans-serif"` | Interface font family list |
| `mono` | `"JetBrains Mono, monospace"` | Monospace font family list |
| `scale` | `1.0` | Multiplies every font size (0.5–2.0) |

## Example

```toml
base = "gruvbox"
name = "My Gruvbox"

[colors]
accent = "#8ec07c"
accent-hover = "#a9d69a"

[files]
folder = "@accent"

[shape]
radius = 4
density = "compact"

[fonts]
ui = "IBM Plex Sans, sans-serif"
scale = 1.05
```

## `user.css`

`user.css` is loaded with the highest priority, so any rule in it wins. Syntax errors are reported
with a line and column, and the rest of the file still applies.

```css
/* Bigger file names in grid view */
.file-card-name { font-size: 14px; }

/* No shadows on cards */
.file-card, .file-card:hover { box-shadow: none; }
```

### Stable CSS classes

| Area | Classes |
|---|---|
| Sidebar | `.sidebar`, `.sidebar-title`, `.place-btn`, `.toolbar`, `.toolbar-btn` |
| Header | `.header-bar`, `.breadcrumb-label`, `.breadcrumb-label-active` |
| Grid | `.file-card`, `.file-card-name`, `.file-card-meta` |
| List | `.file-row`, `.file-row-meta`, `.group-header` |
| Tree | `.tree-row-btn`, `.tree-row-selected`, `.tree-name`, `.tree-name-dir`, `.tree-kind-<kind>`, `.tree-badge`, `.tree-meta` |
| Menus | `.context-menu`, `.context-menu-item`, `.context-menu-danger`, `.context-menu-title` |
| Buttons | `.btn-primary`, `.btn-secondary` |
| Inspector | `.inspector-title`, `.inspector-subtitle` |
| Icons | `.icon-<kind>` (same kinds as `[files]`) |

The full generated stylesheet is built from [`src/theme/base.css`](../src/theme/base.css).
