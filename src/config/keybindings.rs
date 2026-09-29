use std::collections::BTreeMap;
use std::path::Path;

// ═══════════════════════════════════════════════
//  keybindings.toml — keyboard shortcuts
// ═══════════════════════════════════════════════
//
//   [keys]
//   back = "<Alt>Left"                 # one accelerator
//   refresh = ["F5", "<Ctrl>r"]        # several
//   toggle-hidden = []                 # none: removes the default
//
// Only the actions listed in the file change; the rest keep the defaults
// from `BINDABLE`. Accelerator *syntax* is checked by GTK when the file is
// applied (see `ui::shortcuts`), since that needs GTK to be initialized.

pub const KEYBINDINGS_FILE: &str = "keybindings.toml";

/// Where a shortcut is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Anywhere in the window.
    Window,
    /// Only while the file view has focus, so Delete, F2, Escape and Ctrl+A
    /// keep working normally in text fields.
    View,
}

pub struct Bindable {
    /// `win.*` action name; `name::target` for a stateful action's value.
    pub name: &'static str,
    pub title: &'static str,
    pub defaults: &'static [&'static str],
    pub scope: Scope,
}

const fn bind(
    name: &'static str,
    title: &'static str,
    defaults: &'static [&'static str],
    scope: Scope,
) -> Bindable {
    Bindable {
        name,
        title,
        defaults,
        scope,
    }
}

/// Every action a shortcut can trigger, with its default shortcuts.
pub const BINDABLE: &[Bindable] = {
    use Scope::*;
    &[
        bind("back", "Back", &["<Alt>Left", "Back"], Window),
        bind("forward", "Forward", &["<Alt>Right", "Forward"], Window),
        bind("go-up", "Parent folder", &["<Alt>Up"], Window),
        bind("go-home", "Home folder", &["<Alt>Home"], Window),
        bind("refresh", "Refresh", &["F5", "<Ctrl>r"], Window),
        bind("new", "New folder or file", &["<Ctrl><Shift>n"], Window),
        bind("toggle-hidden", "Show hidden files", &["<Ctrl>h"], Window),
        bind("toggle-sidebar", "Toggle sidebar", &["F9"], Window),
        bind("toggle-inspector", "Toggle inspector", &["<Ctrl>i"], Window),
        bind("view-mode::grid", "Grid view", &["<Ctrl>1"], Window),
        bind("view-mode::list", "List view", &["<Ctrl>2"], Window),
        bind("view-mode::tree", "Tree view", &["<Ctrl>3"], Window),
        bind("view-mode::graph", "Graph view", &["<Ctrl>4"], Window),
        bind("cycle-view", "Next view mode", &[], Window),
        bind("show-settings", "Customize", &["<Ctrl>comma"], Window),
        bind(
            "command-palette",
            "Command palette",
            &["<Ctrl><Shift>p", "<Ctrl>k"],
            Window,
        ),
        bind(
            "close-window",
            "Close window",
            &["<Ctrl>w", "<Ctrl>q"],
            Window,
        ),
        bind("about", "About Diptych", &[], Window),
        bind("open-selection", "Open selection", &[], View),
        bind("rename-selection", "Rename", &["F2"], View),
        bind("trash-selection", "Move to trash", &["Delete"], View),
        bind(
            "delete-selection",
            "Delete permanently",
            &["<Shift>Delete"],
            View,
        ),
        bind("select-all", "Select all", &["<Ctrl>a"], View),
        bind("unselect-all", "Clear selection", &["Escape"], View),
    ]
};

pub fn bindable(name: &str) -> Option<&'static Bindable> {
    BINDABLE.iter().find(|b| b.name == name)
}

/// Accelerators per action, defaults merged with the user's file.
#[derive(Debug, Clone, PartialEq)]
pub struct Keybindings {
    keys: BTreeMap<&'static str, Vec<String>>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            keys: BINDABLE
                .iter()
                .map(|b| (b.name, b.defaults.iter().map(|s| s.to_string()).collect()))
                .collect(),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    keys: BTreeMap<String, OneOrMany>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl Keybindings {
    pub fn parse(src: &str) -> Result<Keybindings, String> {
        let file: File = toml::from_str(src).map_err(|e| e.to_string())?;
        let mut out = Keybindings::default();
        for (name, value) in file.keys {
            let b = bindable(&name).ok_or_else(|| format!("unknown action “{}”", name))?;
            let accels = match value {
                OneOrMany::One(s) => vec![s],
                OneOrMany::Many(v) => v,
            };
            if let Some(bad) = accels.iter().find(|a| a.trim().is_empty()) {
                return Err(format!("{}: empty shortcut {:?}", name, bad));
            }
            out.keys.insert(b.name, accels);
        }
        Ok(out)
    }

    /// Reads `keybindings.toml` from `dir`; a missing file means defaults.
    pub fn load(dir: &Path) -> Result<Keybindings, String> {
        match std::fs::read_to_string(dir.join(KEYBINDINGS_FILE)) {
            Ok(src) => Self::parse(&src),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn get(&self, name: &str) -> &[String] {
        self.keys.get(name).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Sets `name`'s shortcuts in a keybindings.toml document. Matching the
/// defaults removes the entry, so the file only lists real changes.
pub fn write_keys(
    doc: &mut toml_edit::DocumentMut,
    name: &str,
    accels: &[String],
) -> Result<(), String> {
    let b = bindable(name).ok_or_else(|| format!("unknown action “{}”", name))?;
    if accels
        .iter()
        .map(String::as_str)
        .eq(b.defaults.iter().copied())
    {
        super::edit::remove(doc, Some("keys"), name);
        // `remove` drops an emptied [keys]; keep the header for hand edits.
        super::edit::table(doc, "keys")?;
        return Ok(());
    }
    let value = match accels {
        [one] => toml_edit::Value::from(one.as_str()),
        many => super::edit::string_array(many),
    };
    super::edit::set(doc, Some("keys"), name, value)
}

/// Writes a starter `keybindings.toml` (every default, commented out) on
/// first run.
pub fn seed(dir: &Path) -> std::io::Result<()> {
    let path = dir.join(KEYBINDINGS_FILE);
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(path, starter_keybindings())
}

pub fn starter_keybindings() -> String {
    let mut s = String::from(
        "# Diptych keyboard shortcuts — changes apply as soon as you save.\n\
         #\n\
         # Uncomment a line to change it. A value is one shortcut or a list;\n\
         # [] removes the shortcut. Modifiers: <Ctrl> <Shift> <Alt> <Super>.\n\
         # Key names are GDK names: a, F5, Delete, Left, Home, comma, space, …\n\
         # Shortcuts for your own commands go in actions.toml (`key = …`).\n\
         \n\
         [keys]\n",
    );
    for b in BINDABLE {
        let value = match b.defaults {
            [] => "[]".to_string(),
            [one] => format!("{:?}", one),
            many => format!("{:?}", many),
        };
        let line = format!("# {} = {}", toml_key(b.name), value);
        let note = match b.scope {
            Scope::Window => b.title.to_string(),
            Scope::View => format!("{} (file view focused)", b.title),
        };
        s.push_str(&format!("{:<46}  # {}\n", line, note));
    }
    s
}

/// `view-mode::grid` isn't a bare TOML key.
fn toml_key(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        name.to_string()
    } else {
        format!("{:?}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_file_equals_defaults() {
        assert_eq!(
            Keybindings::parse(&starter_keybindings()).unwrap(),
            Keybindings::default()
        );
        // Uncommenting every line also gives the defaults.
        let uncommented: String = starter_keybindings()
            .lines()
            .map(|l| {
                let binding = |l: &&str| {
                    let key = l.split(" = ").next().unwrap_or("").trim_matches('"');
                    bindable(key).is_some()
                };
                let l = l.strip_prefix("# ").filter(binding).unwrap_or(l);
                format!("{}\n", l.split("  # ").next().unwrap())
            })
            .collect();
        assert_eq!(
            Keybindings::parse(&uncommented).unwrap(),
            Keybindings::default()
        );
    }

    #[test]
    fn overrides_merge_with_defaults() {
        let k = Keybindings::parse(
            "[keys]\nback = \"BackSpace\"\nrefresh = [\"F5\"]\n\"view-mode::grid\" = []",
        )
        .unwrap();
        assert_eq!(k.get("back"), ["BackSpace"]);
        assert_eq!(k.get("refresh"), ["F5"]);
        assert!(k.get("view-mode::grid").is_empty());
        assert_eq!(k.get("forward"), ["<Alt>Right", "Forward"]);
    }

    #[test]
    fn rejects_mistakes() {
        let err = |s: &str| Keybindings::parse(s).unwrap_err();
        assert!(err("[keys]\nbakc = \"F1\"").contains("unknown action “bakc”"));
        assert!(err("[keys]\nback = \"\"").contains("empty shortcut"));
        assert!(err("[key]\nback = \"F1\"").contains("unknown field"));
        assert!(!err("[keys]\nback = 3").is_empty());
    }

    #[test]
    fn write_keys_round_trips() {
        let edit = |src: &str, name: &str, accels: &[&str]| {
            let accels: Vec<String> = accels.iter().map(|s| s.to_string()).collect();
            crate::config::edit::edit_str(src, |d| write_keys(d, name, &accels)).unwrap()
        };
        let src = starter_keybindings();
        let out = edit(&src, "back", &["BackSpace"]);
        let out = edit(&out, "view-mode::grid", &[]);
        let out = edit(&out, "refresh", &["F5", "<Ctrl>F5"]);
        let k = Keybindings::parse(&out).unwrap();
        assert_eq!(k.get("back"), ["BackSpace"]);
        assert!(k.get("view-mode::grid").is_empty());
        assert_eq!(k.get("refresh"), ["F5", "<Ctrl>F5"]);
        // The documentation comments survive.
        assert!(out.contains("# Uncomment a line to change it."));

        // Back to the defaults: the entries disappear again.
        let out = edit(&out, "back", &["<Alt>Left", "Back"]);
        let out = edit(&out, "view-mode::grid", &["<Ctrl>1"]);
        let out = edit(&out, "refresh", &["F5", "<Ctrl>r"]);
        assert_eq!(Keybindings::parse(&out).unwrap(), Keybindings::default());
        assert!(!out.lines().any(|l| l.starts_with("back =")), "{}", out);
    }

    #[test]
    fn names_are_unique() {
        for (i, b) in BINDABLE.iter().enumerate() {
            assert!(BINDABLE[..i].iter().all(|o| o.name != b.name), "{}", b.name);
        }
    }
}
