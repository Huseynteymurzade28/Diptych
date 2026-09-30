use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════
//  Copy / Move (paste, drag and drop)
// ═══════════════════════════════════════════════
//
// Blocking; the UI runs it with `gio::spawn_blocking`. Existing files are
// never overwritten: a clash gets a free name ("notes (2).txt").

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Copy,
    Move,
}

/// What happened to one source item.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Done(PathBuf),
    /// Moving an item into the folder it's already in does nothing.
    Unchanged,
    Failed(String),
}

/// Copies or moves each of `sources` into `dest_dir`.
pub fn transfer(sources: &[PathBuf], dest_dir: &Path, mode: Mode) -> Vec<(PathBuf, Outcome)> {
    sources
        .iter()
        .map(|src| (src.clone(), transfer_one(src, dest_dir, mode)))
        .collect()
}

fn transfer_one(src: &Path, dest_dir: &Path, mode: Mode) -> Outcome {
    let Some(name) = src.file_name() else {
        return Outcome::Failed("can’t copy the root folder".into());
    };
    if dest_dir.starts_with(src) {
        return Outcome::Failed("can’t put a folder inside itself".into());
    }
    if mode == Mode::Move && src.parent() == Some(dest_dir) {
        return Outcome::Unchanged;
    }
    let target = free_name(dest_dir, &name.to_string_lossy(), |p| {
        fs::symlink_metadata(p).is_ok()
    });
    let result = match mode {
        Mode::Copy => copy_recursive(src, &target),
        Mode::Move => fs::rename(src, &target).or_else(|e| {
            // Another filesystem: copy, then remove the original.
            if e.kind() == io::ErrorKind::CrossesDevices {
                copy_recursive(src, &target).and_then(|()| remove_recursive(src))
            } else {
                Err(e)
            }
        }),
    };
    match result {
        Ok(()) => Outcome::Done(target),
        Err(e) => Outcome::Failed(e.to_string()),
    }
}

/// `dir/name`, or `dir/stem (2).ext`, `(3)`… if that's taken.
pub fn free_name(dir: &Path, name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(name);
    if !exists(&first) {
        return first;
    }
    // Hidden files without extension (".bashrc") and folders keep the
    // whole name as the stem.
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..)
        .map(|n| dir.join(format!("{} ({}){}", stem, n, ext)))
        .find(|p| !exists(p))
        .expect("some number is free")
}

/// Copies a file, symlink or folder tree. Symlinks are copied as links.
fn copy_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(src)?, dest)
    } else if meta.is_dir() {
        fs::create_dir(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
        fs::set_permissions(dest, meta.permissions())
    } else {
        fs::copy(src, dest).map(drop)
    }
}

fn remove_recursive(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("diptych-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn free_names() {
        let taken = |names: &'static [&str]| move |p: &Path| names.iter().any(|n| p.ends_with(n));
        let dir = Path::new("/d");
        assert_eq!(free_name(dir, "a.txt", taken(&[])), dir.join("a.txt"));
        assert_eq!(
            free_name(dir, "a.txt", taken(&["a.txt", "a (2).txt"])),
            dir.join("a (3).txt")
        );
        assert_eq!(
            free_name(dir, "Photos", taken(&["Photos"])),
            dir.join("Photos (2)")
        );
        assert_eq!(
            free_name(dir, ".bashrc", taken(&[".bashrc"])),
            dir.join(".bashrc (2)")
        );
        assert_eq!(
            free_name(dir, "x.tar.gz", taken(&["x.tar.gz"])),
            dir.join("x.tar (2).gz")
        );
    }

    #[test]
    fn copies_trees_and_moves() {
        let root = scratch("transfer");
        let src = root.join("src");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("sub/f.txt"), "hi").unwrap();
        std::os::unix::fs::symlink("sub/f.txt", src.join("link")).unwrap();
        let dest = root.join("dest");
        fs::create_dir(&dest).unwrap();

        // Copy twice: the second gets a free name.
        let out = transfer(std::slice::from_ref(&src), &dest, Mode::Copy);
        assert_eq!(out[0].1, Outcome::Done(dest.join("src")));
        assert_eq!(
            fs::read_to_string(dest.join("src/sub/f.txt")).unwrap(),
            "hi"
        );
        assert!(fs::symlink_metadata(dest.join("src/link"))
            .unwrap()
            .file_type()
            .is_symlink());
        let out = transfer(std::slice::from_ref(&src), &dest, Mode::Copy);
        assert_eq!(out[0].1, Outcome::Done(dest.join("src (2)")));

        // Into itself: refused.
        let out = transfer(std::slice::from_ref(&src), &src.join("sub"), Mode::Copy);
        assert!(matches!(out[0].1, Outcome::Failed(_)));

        // Move within the same folder: nothing to do. Move elsewhere: gone from src.
        let file = src.join("sub/f.txt");
        assert_eq!(
            transfer(std::slice::from_ref(&file), &src.join("sub"), Mode::Move)[0].1,
            Outcome::Unchanged
        );
        let out = transfer(std::slice::from_ref(&file), &dest, Mode::Move);
        assert_eq!(out[0].1, Outcome::Done(dest.join("f.txt")));
        assert!(!file.exists());

        fs::remove_dir_all(&root).unwrap();
    }
}
