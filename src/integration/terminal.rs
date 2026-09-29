use std::path::Path;

// ═══════════════════════════════════════════════
//  Which terminal to open
// ═══════════════════════════════════════════════
//
// "Open Terminal Here" tries, in order:
//   1. xdg-terminal-exec — the freedesktop way to pick the user's terminal
//   2. $TERMINAL
//   3. the desktop's own terminal (Ptyxis / Console on GNOME, Konsole on KDE…)
//   4. a list of common terminals
// Every candidate starts in the folder (its working directory); terminals
// that ignore that get their "start here" flag.

/// Terminals Diptych knows, each with the arguments that make it start in
/// `dir` beyond the inherited working directory.
fn dir_args(program: &str, dir: &str) -> Vec<String> {
    let name = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);
    match name {
        // Client/server terminals: the server's directory would win.
        "gnome-terminal" | "kgx" | "xfce4-terminal" | "mate-terminal" => {
            vec![format!("--working-directory={}", dir)]
        }
        "ptyxis" => vec![
            "--new-window".into(),
            format!("--working-directory={}", dir),
        ],
        "konsole" => vec!["--workdir".into(), dir.into()],
        "wezterm" => vec!["start".into(), "--cwd".into(), dir.into()],
        _ => vec![],
    }
}

fn desktop_defaults(desktop: &str) -> &'static [&'static str] {
    let is = |name: &str| {
        desktop
            .split(':')
            .any(|d| d.trim().eq_ignore_ascii_case(name))
    };
    if is("GNOME") || is("Unity") || is("Pantheon") {
        &["ptyxis", "kgx", "gnome-terminal"]
    } else if is("KDE") {
        &["konsole"]
    } else if is("XFCE") {
        &["xfce4-terminal"]
    } else if is("MATE") {
        &["mate-terminal"]
    } else {
        &[]
    }
}

const COMMON: &[&str] = &[
    "kitty",
    "foot",
    "alacritty",
    "wezterm",
    "ghostty",
    "konsole",
    "ptyxis",
    "kgx",
    "gnome-terminal",
    "xfce4-terminal",
    "xterm",
];

/// The command to open a terminal in `dir`, or `None` if nothing usable is
/// installed. `desktop` is `$XDG_CURRENT_DESKTOP`, `terminal` is
/// `$TERMINAL`, and `installed` says whether a program is in `$PATH`.
pub fn command(
    desktop: &str,
    terminal: Option<&str>,
    dir: &Path,
    installed: impl Fn(&str) -> bool,
) -> Option<Vec<String>> {
    let dir = dir.to_string_lossy();
    if installed("xdg-terminal-exec") {
        // Uses the working directory (and `--dir` on newer versions,
        // which older ones would treat as the command to run).
        return Some(vec!["xdg-terminal-exec".into()]);
    }
    // $TERMINAL may carry arguments: "kitty --single-instance".
    if let Some(words) = terminal
        .and_then(|t| glib::shell_parse_argv(t).ok())
        .map(|w| {
            w.into_iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        })
        .filter(|w| w.first().is_some_and(|p| installed(p)))
    {
        let extra = dir_args(&words[0], &dir);
        return Some(words.into_iter().chain(extra).collect());
    }
    desktop_defaults(desktop)
        .iter()
        .chain(COMMON)
        .find(|t| installed(t))
        .map(|t| {
            std::iter::once(t.to_string())
                .chain(dir_args(t, &dir))
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(desktop: &str, terminal: Option<&str>, have: &[&str]) -> Option<Vec<String>> {
        command(desktop, terminal, Path::new("/home/u/My Files"), |p| {
            have.contains(&p)
        })
    }

    #[test]
    fn prefers_xdg_terminal_exec_then_terminal_env() {
        assert_eq!(
            run(
                "KDE",
                Some("kitty"),
                &["xdg-terminal-exec", "kitty", "konsole"]
            )
            .unwrap(),
            ["xdg-terminal-exec"]
        );
        assert_eq!(
            run(
                "KDE",
                Some("kitty --single-instance"),
                &["kitty", "konsole"]
            )
            .unwrap(),
            ["kitty", "--single-instance"]
        );
        // $TERMINAL pointing at something that isn't installed is skipped.
        assert_eq!(
            run("KDE", Some("nope"), &["konsole"]).unwrap(),
            ["konsole", "--workdir", "/home/u/My Files"]
        );
    }

    #[test]
    fn falls_back_to_the_desktop_then_common_terminals() {
        assert_eq!(
            run("ubuntu:GNOME", None, &["kgx", "gnome-terminal", "kitty"]).unwrap(),
            ["kgx", "--working-directory=/home/u/My Files"]
        );
        assert_eq!(
            run("GNOME", None, &["ptyxis"]).unwrap(),
            [
                "ptyxis",
                "--new-window",
                "--working-directory=/home/u/My Files"
            ]
        );
        assert_eq!(
            run("Hyprland", None, &["foot", "kitty"]).unwrap(),
            ["kitty"]
        );
        assert_eq!(run("Hyprland", None, &["foot"]).unwrap(), ["foot"]);
        assert!(run("sway", None, &[]).is_none());
    }
}
