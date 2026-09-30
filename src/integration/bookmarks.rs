use gio::prelude::*;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════
//  Bookmarks — the GTK bookmarks file
// ═══════════════════════════════════════════════
//
// `~/.config/gtk-3.0/bookmarks` is shared by Nautilus, the GTK file chooser
// and other GTK apps, so a folder pinned in one shows up in all of them.
// One bookmark per line: a URI, optionally followed by a space and a label.
//
//   file:///home/u/Projects
//   file:///home/u/Pictures/Wallpapers Walls
//   sftp://server/srv            ← not a local folder: kept, not shown

#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    pub path: PathBuf,
    /// The custom label, if any; otherwise the folder name is shown.
    pub label: Option<String>,
}

impl Bookmark {
    pub fn title(&self) -> String {
        self.label.clone().unwrap_or_else(|| {
            self.path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| self.path.display().to_string())
        })
    }
}

pub fn file() -> PathBuf {
    glib::user_config_dir().join("gtk-3.0").join("bookmarks")
}

/// Local folders bookmarked in `src`, in order.
pub fn parse(src: &str) -> Vec<Bookmark> {
    src.lines()
        .filter_map(|line| {
            let line = line.trim();
            let (uri, label) = match line.split_once(' ') {
                Some((uri, label)) => (uri, Some(label.trim().to_string())),
                None => (line, None),
            };
            if !uri.starts_with("file://") {
                return None;
            }
            let path = gio::File::for_uri(uri).path()?;
            Some(Bookmark {
                path,
                label: label.filter(|l| !l.is_empty()),
            })
        })
        .collect()
}

/// `src` with `path` appended, unless it's already bookmarked.
pub fn with_added(src: &str, path: &Path) -> Option<String> {
    if parse(src).iter().any(|b| b.path == path) {
        return None;
    }
    let mut out = src.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&gio::File::for_path(path).uri());
    out.push('\n');
    Some(out)
}

/// `src` without the lines that bookmark `path`; other lines, including
/// ones Diptych can't show (remote URIs), are kept as they are.
pub fn with_removed(src: &str, path: &Path) -> String {
    src.lines()
        .filter(|line| parse(line).first().is_none_or(|b| b.path != path))
        .map(|line| format!("{}\n", line))
        .collect()
}

pub fn load() -> Vec<Bookmark> {
    std::fs::read_to_string(file())
        .map(|s| parse(&s))
        .unwrap_or_default()
}

pub fn add(path: &Path) -> std::io::Result<bool> {
    let file = file();
    let src = std::fs::read_to_string(&file).unwrap_or_default();
    match with_added(&src, path) {
        Some(out) => {
            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&file, out)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

pub fn remove(path: &Path) -> std::io::Result<()> {
    let file = file();
    let src = std::fs::read_to_string(&file)?;
    std::fs::write(&file, with_removed(&src, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "file:///home/u/Projects\n\
                       file:///home/u/My%20Pictures Walls\n\
                       sftp://server/srv\n";

    #[test]
    fn parses_local_bookmarks_with_labels() {
        let b = parse(SRC);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].path, PathBuf::from("/home/u/Projects"));
        assert_eq!(b[0].title(), "Projects");
        assert_eq!(b[1].path, PathBuf::from("/home/u/My Pictures"));
        assert_eq!(b[1].title(), "Walls");
    }

    #[test]
    fn adds_once_and_encodes() {
        let out = with_added(SRC, Path::new("/home/u/a b")).unwrap();
        assert!(out.ends_with("file:///home/u/a%20b\n"));
        assert!(with_added(&out, Path::new("/home/u/a b")).is_none());
        assert_eq!(
            with_added("", Path::new("/x")).as_deref(),
            Some("file:///x\n")
        );
    }

    #[test]
    fn removes_only_that_folder() {
        let out = with_removed(SRC, Path::new("/home/u/My Pictures"));
        assert_eq!(out, "file:///home/u/Projects\nsftp://server/srv\n");
    }
}
