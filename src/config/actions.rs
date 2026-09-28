use serde::Deserialize;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════
//  actions.toml — your own commands
// ═══════════════════════════════════════════════
//
//   [[action]]
//   name    = "Open in terminal"
//   command = "kitty --directory {dir}"
//   when    = "any"              # any | none | file | folder | ext:png,jpg
//   key     = "<Ctrl><Alt>t"     # optional
//
// Commands are split like a shell would (quotes work) but run directly,
// without a shell, so file names are never re-interpreted. `shell = true`
// runs `sh -c` instead, with every placeholder shell-quoted.

pub const ACTIONS_FILE: &str = "actions.toml";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Actions(pub Vec<CustomAction>);

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomAction {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub when: When,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub shell: bool,
    /// Icon name for menus (e.g. "utilities-terminal-symbolic").
    #[serde(default)]
    pub icon: Option<String>,
}

/// Which selections an action applies to.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(try_from = "String")]
pub enum When {
    /// Always (with nothing selected, it acts on the folder).
    #[default]
    Any,
    /// Only with nothing selected (the folder's background menu).
    None,
    /// One or more files, no folders.
    File,
    /// One or more folders, no files.
    Folder,
    /// Files whose extensions are all in the list (lowercase, no dot).
    Ext(Vec<String>),
}

impl TryFrom<String> for When {
    type Error = String;
    fn try_from(s: String) -> Result<When, String> {
        Ok(match s.trim() {
            "any" => When::Any,
            "none" => When::None,
            "file" => When::File,
            "folder" => When::Folder,
            other => {
                let list = other.strip_prefix("ext:").ok_or_else(|| {
                    format!(
                        "when = “{}”: expected any, none, file, folder or ext:png,jpg",
                        other
                    )
                })?;
                let exts: Vec<String> = list
                    .split(',')
                    .map(|e| e.trim().trim_start_matches('.').to_lowercase())
                    .filter(|e| !e.is_empty())
                    .collect();
                if exts.is_empty() {
                    return Err("when = “ext:”: list at least one extension".into());
                }
                When::Ext(exts)
            }
        })
    }
}

/// What an action runs on.
pub struct Target<'a> {
    pub dir: &'a Path,
    /// Selected paths, each with whether it's a folder.
    pub selection: &'a [(PathBuf, bool)],
}

const PLACEHOLDERS: [&str; 4] = ["{files}", "{file}", "{dir}", "{name}"];

impl CustomAction {
    /// Whether the action shows up / runs for `target`.
    pub fn applies_to(&self, target: &Target) -> bool {
        let sel = target.selection;
        let when = match &self.when {
            When::Any => true,
            When::None => sel.is_empty(),
            When::File => !sel.is_empty() && sel.iter().all(|(_, dir)| !dir),
            When::Folder => !sel.is_empty() && sel.iter().all(|(_, dir)| *dir),
            When::Ext(exts) => {
                !sel.is_empty()
                    && sel.iter().all(|(p, dir)| {
                        !dir && p
                            .extension()
                            .map(|e| exts.contains(&e.to_string_lossy().to_lowercase()))
                            .unwrap_or(false)
                    })
            }
        };
        when && (!sel.is_empty() || !self.needs_selection())
    }

    fn needs_selection(&self) -> bool {
        ["{file}", "{files}", "{name}"]
            .iter()
            .any(|p| self.command.contains(p))
    }

    /// The program and arguments to run for `target`.
    pub fn argv(&self, target: &Target) -> Result<Vec<String>, String> {
        let paths: Vec<String> = target
            .selection
            .iter()
            .map(|(p, _)| p.to_string_lossy().into_owned())
            .collect();
        let first = target.selection.first().map(|(p, _)| p.as_path());
        let value = |ph: &str, quote: bool| -> String {
            let q = |s: String| {
                if quote {
                    glib::shell_quote(&s).to_string_lossy().into_owned()
                } else {
                    s
                }
            };
            match ph {
                "{files}" => paths.iter().cloned().map(q).collect::<Vec<_>>().join(" "),
                "{file}" => q(first
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default()),
                "{name}" => q(first
                    .and_then(Path::file_name)
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()),
                _ => q(target.dir.to_string_lossy().into_owned()),
            }
        };

        if self.shell {
            let script = expand(&self.command, |ph| value(ph, true));
            return Ok(vec!["sh".into(), "-c".into(), script]);
        }
        let words = glib::shell_parse_argv(&self.command)
            .map_err(|e| format!("{}: {}", self.name, e.message()))?;
        let mut argv = vec![];
        for word in words {
            let word = word.to_string_lossy();
            if word == "{files}" {
                // A bare {files} becomes one argument per file.
                argv.extend(paths.iter().cloned());
            } else {
                argv.push(expand(&word, |ph| value(ph, false)));
            }
        }
        Ok(argv)
    }
}

/// Replaces placeholders in one pass, so a file named "{dir}" stays as is.
fn expand(s: &str, value: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut rest = s;
    'scan: while !rest.is_empty() {
        for ph in PLACEHOLDERS {
            if let Some(after) = rest.strip_prefix(ph) {
                out.push_str(&value(ph));
                rest = after;
                continue 'scan;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    action: Vec<CustomAction>,
}

impl Actions {
    pub fn parse(src: &str) -> Result<Actions, String> {
        let file: File = toml::from_str(src).map_err(|e| e.to_string())?;
        for (i, a) in file.action.iter().enumerate() {
            if a.name.trim().is_empty() {
                return Err(format!("action #{} has an empty name", i + 1));
            }
            if a.command.trim().is_empty() {
                return Err(format!("“{}” has an empty command", a.name));
            }
            if !a.shell {
                glib::shell_parse_argv(&a.command)
                    .map_err(|e| format!("“{}”: {}", a.name, e.message()))?;
            }
            if a.key.as_deref().is_some_and(|k| k.trim().is_empty()) {
                return Err(format!("“{}” has an empty key", a.name));
            }
        }
        Ok(Actions(file.action))
    }

    /// Reads `actions.toml` from `dir`; a missing file means no actions.
    pub fn load(dir: &Path) -> Result<Actions, String> {
        match std::fs::read_to_string(dir.join(ACTIONS_FILE)) {
            Ok(src) => Self::parse(&src),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Writes a commented starter `actions.toml` on first run.
pub fn seed(dir: &Path) -> std::io::Result<()> {
    let path = dir.join(ACTIONS_FILE);
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(path, STARTER_ACTIONS)
}

pub const STARTER_ACTIONS: &str = r##"# Diptych custom actions — changes apply as soon as you save.
#
# Each [[action]] appears in the right-click menu when `when` matches
# the selection, and runs on its `key` shortcut.
#
#   name     menu label
#   command  program and arguments; quotes work like in a shell, but no
#            shell runs, so file names can't break the command.
#            Placeholders: {file} first selected item, {files} every
#            selected item (one argument each), {name} first item's file
#            name, {dir} the current folder.
#   when     any (default) | none (nothing selected) | file | folder |
#            ext:png,jpg (only files with these extensions)
#   key      optional shortcut, e.g. "<Ctrl><Alt>t"
#   shell    true runs the command with `sh -c` (pipes, &&, …); the
#            placeholders are quoted for you.
#   icon     optional icon name, e.g. "utilities-terminal-symbolic"
#
# Examples — remove the leading "# " to enable one:

# [[action]]
# name = "Open Terminal Here"
# command = "kitty --directory {dir}"
# when = "none"
# key = "<Ctrl><Alt>t"
# icon = "utilities-terminal-symbolic"

# [[action]]
# name = "Copy Paths"
# command = 'printf "%s\n" {files} | wl-copy'
# shell = true

# [[action]]
# name = "Convert to PNG"
# command = "magick {file} {file}.png"
# when = "ext:jpg,jpeg,webp,heic"
"##;

#[cfg(test)]
mod tests {
    use super::*;

    fn action(command: &str) -> CustomAction {
        CustomAction {
            name: "t".into(),
            command: command.into(),
            when: When::Any,
            key: None,
            shell: false,
            icon: None,
        }
    }

    fn sel(items: &[(&str, bool)]) -> Vec<(PathBuf, bool)> {
        items.iter().map(|(p, d)| (PathBuf::from(p), *d)).collect()
    }

    #[test]
    fn starter_file_has_no_active_actions() {
        assert_eq!(Actions::parse(STARTER_ACTIONS).unwrap(), Actions::default());
        // Every example is valid once uncommented.
        let uncommented: String = STARTER_ACTIONS
            .lines()
            .filter(|l| l.starts_with("# [[") || l.starts_with("# ") && l.contains(" = "))
            .map(|l| format!("{}\n", &l[2..]))
            .collect();
        assert_eq!(Actions::parse(&uncommented).unwrap().0.len(), 3);
    }

    #[test]
    fn parses_when() {
        let a =
            Actions::parse("[[action]]\nname = \"x\"\ncommand = \"x\"\nwhen = \"ext: PNG, .jpg\"")
                .unwrap();
        assert_eq!(a.0[0].when, When::Ext(vec!["png".into(), "jpg".into()]));
        let err = |s: &str| Actions::parse(s).unwrap_err();
        assert!(
            err("[[action]]\nname = \"x\"\ncommand = \"x\"\nwhen = \"files\"").contains("when")
        );
        assert!(
            err("[[action]]\nname = \"x\"\ncommand = \"x\"\nwhen = \"ext:\"").contains("extension")
        );
        assert!(err("[[action]]\nname = \"\"\ncommand = \"x\"").contains("empty name"));
        assert!(err("[[action]]\nname = \"x\"\ncommand = \"a 'b\"").contains("“x”"));
        assert!(
            err("[[action]]\nname = \"x\"\ncommand = \"x\"\nkye = \"F1\"")
                .contains("unknown field")
        );
    }

    #[test]
    fn applies_by_selection() {
        let dir = Path::new("/d");
        let check = |a: &CustomAction, s: &[(&str, bool)]| {
            a.applies_to(&Target {
                dir,
                selection: &sel(s),
            })
        };
        let mut a = action("echo {dir}");
        assert!(check(&a, &[]));
        assert!(check(&a, &[("/d/a", false)]));

        a.when = When::None;
        assert!(check(&a, &[]));
        assert!(!check(&a, &[("/d/a", false)]));

        a.when = When::File;
        assert!(!check(&a, &[]));
        assert!(check(&a, &[("/d/a", false), ("/d/b", false)]));
        assert!(!check(&a, &[("/d/a", false), ("/d/b", true)]));

        a.when = When::Folder;
        assert!(check(&a, &[("/d/b", true)]));
        assert!(!check(&a, &[("/d/a", false)]));

        a.when = When::Ext(vec!["png".into()]);
        assert!(check(&a, &[("/d/a.PNG", false)]));
        assert!(!check(&a, &[("/d/a.png", false), ("/d/b.jpg", false)]));
        assert!(!check(&a, &[("/d/x.png", true)]));

        // {file} needs a selection even with when = any.
        let a = action("xdg-open {file}");
        assert!(!check(&a, &[]));
        assert!(check(&a, &[("/d/a", false)]));
    }

    #[test]
    fn expands_without_shell() {
        let s = sel(&[("/d/it's a file.txt", false), ("/d/{dir}", false)]);
        let t = Target {
            dir: Path::new("/d"),
            selection: &s,
        };
        assert_eq!(
            action("tool --in={file} {files} 'name: {name}' -C {dir}")
                .argv(&t)
                .unwrap(),
            [
                "tool",
                "--in=/d/it's a file.txt",
                "/d/it's a file.txt",
                "/d/{dir}",
                "name: it's a file.txt",
                "-C",
                "/d",
            ]
        );
    }

    #[test]
    fn quotes_for_shell() {
        let s = sel(&[("/d/it's.txt", false), ("/d/$(rm -rf ~)", false)]);
        let t = Target {
            dir: Path::new("/d"),
            selection: &s,
        };
        let mut a = action("printf %s {files} | wl-copy");
        a.shell = true;
        let argv = a.argv(&t).unwrap();
        assert_eq!(argv[..2], ["sh", "-c"]);
        assert_eq!(
            argv[2],
            r#"printf %s '/d/it'\''s.txt' '/d/$(rm -rf ~)' | wl-copy"#
        );
    }
}
