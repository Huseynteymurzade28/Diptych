// ─── Desktop Integration ───
// Opening folders from other apps (command line, `inode/directory`,
// org.freedesktop.FileManager1) and launching the user's terminal.

pub mod file_manager1;
pub mod terminal;

use std::path::PathBuf;

/// Windows to open for some paths: each folder as itself, each file (or,
/// with `reveal`, every path) in its parent folder with it selected.
/// Order is kept and items sharing a folder share a window.
pub fn windows_for(paths: &[PathBuf], reveal: bool) -> Vec<(PathBuf, Vec<PathBuf>)> {
    let mut out: Vec<(PathBuf, Vec<PathBuf>)> = vec![];
    for path in paths {
        let (dir, item) = if path.is_dir() && !reveal {
            (path.clone(), None)
        } else {
            match path.parent() {
                Some(parent) if path.exists() => (parent.to_path_buf(), Some(path.clone())),
                // A folder that doesn't exist (or "/"): open what we can.
                _ if path.is_dir() => (path.clone(), None),
                _ => continue,
            }
        };
        match out.iter_mut().find(|(d, _)| *d == dir) {
            Some((_, items)) => items.extend(item),
            None => out.push((dir, item.into_iter().collect())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::windows_for;
    use std::path::PathBuf;

    #[test]
    fn groups_by_folder() {
        let dir = std::env::temp_dir().join(format!("diptych-int-{}", std::process::id()));
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        let missing = dir.join("missing");

        let open = windows_for(&[sub.clone(), a.clone(), missing.clone(), b.clone()], false);
        assert_eq!(
            open,
            [
                (sub.clone(), vec![]),
                (dir.clone(), vec![a.clone(), b.clone()])
            ]
        );
        // Revealing a folder selects it in its parent.
        let reveal = windows_for(&[sub.clone(), a.clone()], true);
        assert_eq!(reveal, [(dir.clone(), vec![sub, a])]);
        assert_eq!(
            windows_for(&[PathBuf::from("/")], true),
            [(PathBuf::from("/"), vec![])]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
