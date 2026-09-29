# Shortcuts and custom actions

Two files in `~/.config/diptych/`, both created on first launch with every option documented. Changes
apply **as soon as you save**. If a file has a mistake, the previous settings stay and a message at
the bottom of the window says what's wrong. It goes away once the file is fixed.

## `keybindings.toml`

```toml
[keys]
back = "BackSpace"                 # one shortcut
refresh = ["F5", "<Ctrl>r"]        # several
toggle-hidden = []                 # none
"view-mode::list" = "<Ctrl>l"      # names with "::" need quotes
```

Only the actions you list change. The rest keep their defaults.

A shortcut is modifiers followed by a GDK key name: `<Ctrl>`, `<Shift>`, `<Alt>`, `<Super>`, then
`a`, `F5`, `Delete`, `Left`, `Home`, `comma`, `space`, `BackSpace`… If a shortcut can't be parsed, or two
actions use the same one, Diptych keeps the first and reports the other.

| Action | Default | |
|---|---|---|
| `back` | `<Alt>Left`, `Back` | |
| `forward` | `<Alt>Right`, `Forward` | |
| `go-up` | `<Alt>Up` | Parent folder |
| `go-home` | `<Alt>Home` | |
| `refresh` | `F5`, `<Ctrl>r` | |
| `new` | `<Ctrl><Shift>n` | New folder or file |
| `toggle-hidden` | `<Ctrl>h` | Show hidden files |
| `toggle-sidebar` | `F9` | |
| `toggle-inspector` | `<Ctrl>i` | |
| `view-mode::grid` / `::list` / `::tree` / `::graph` | `<Ctrl>1` … `<Ctrl>4` | |
| `cycle-view` | — | Next view mode |
| `show-settings` | `<Ctrl>comma` | Customize dialog |
| `command-palette` | `<Ctrl><Shift>p`, `<Ctrl>k` | Command palette |
| `close-window` | `<Ctrl>w`, `<Ctrl>q` | |
| `about` | — | |
| `open-selection` | — | ¹ |
| `rename-selection` | `F2` | ¹ |
| `trash-selection` | `Delete` | ¹ |
| `delete-selection` | `<Shift>Delete` | ¹ Asks first |
| `select-all` | `<Ctrl>a` | ¹ |
| `unselect-all` | `Escape` | ¹ |

¹ Active while the file view has focus, so these keys still edit text in the path bar and name fields.

`F10` always opens the main menu. Header bar tooltips show the current shortcuts.

You don't have to edit the file by hand: **Customize → Shortcuts** (`Ctrl+,`) lists every action.
Click one and press the new keys; if another action already uses them, it gives them up. Changes are
written back to `keybindings.toml`. Only the actions that differ from the default get a line, and
your comments stay.

## Command palette

`Ctrl+Shift+P` (or `Ctrl+K`) opens a searchable list of every action above plus your custom actions
that apply to the current selection. Type a few letters ("shf" finds "Show hidden files"), move
with `↑`/`↓` and press `Enter`.

## `actions.toml`

```toml
[[action]]
name    = "Open Terminal Here"
command = "kitty --directory {dir}"
when    = "none"
key     = "<Ctrl><Alt>t"
icon    = "utilities-terminal-symbolic"

[[action]]
name    = "Convert to PNG"
command = "magick {file} {file}.png"
when    = "ext:jpg,jpeg,webp,heic"

[[action]]
name    = "Copy Paths"
command = 'printf "%s\n" {files} | wl-copy'
shell   = true
```

| Key | Default | Meaning |
|---|---|---|
| `name` | required | Menu label |
| `command` | required | What to run, in the current folder. See below. |
| `when` | `"any"` | `any`, `none` (nothing selected: the folder's background menu), `file` (only files selected), `folder` (only folders), or `ext:png,jpg` (only files with these extensions, any case) |
| `key` | — | Shortcut, same syntax as keybindings.toml |
| `shell` | `false` | Run with `sh -c`, for pipes, `&&`, redirects… |
| `icon` | `system-run-symbolic` | Icon in the background menu |

**Customize → Actions** adds, edits and deletes actions without touching the file (comments and the
commented-out examples stay).

Actions show up in the right-click menu and the command palette when `when` matches: on items for the selection, on empty
space for the folder (right-clicking empty space clears the selection).

### Placeholders

| | |
|---|---|
| `{file}` | First selected item |
| `{files}` | Every selected item. On its own it becomes one argument per item. |
| `{name}` | First selected item's file name |
| `{dir}` | Current folder |

An action that uses `{file}`, `{files}` or `{name}` only applies when something is selected.

Commands are split into words like a shell would (quotes work) but **no shell runs**, so a file name
with spaces, quotes or `$(…)` stays one literal argument. With `shell = true` the placeholders are
shell-quoted for you, so don't put quotes around them.

If the program can't be started or exits with an error, Diptych says so at the bottom of the window.
Its output goes to Diptych's terminal, if it has one.
