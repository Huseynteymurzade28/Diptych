use crate::filesystem::transfer::{self, Mode, Outcome};
use crate::ui::state::AppState;
use adw::prelude::*;
use gtk4::gdk;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Clipboard, paste and drop
// ═══════════════════════════════════════════════
//
// Copy / Cut put the selection on the clipboard in the formats other file
// managers understand: `x-special/gnome-copied-files` (Nautilus, Nemo,
// Thunar; says whether it was a cut), a GdkFileList (`text/uri-list`) and
// plain text paths (for terminals and editors). Paste reads the same, so
// files copied in Dolphin or Nautilus paste here and the other way round.

const GNOME_COPIED: &str = "x-special/gnome-copied-files";

/// Time a transfer may take before a "Copying…" toast shows up.
const PROGRESS_DELAY: std::time::Duration = std::time::Duration::from_millis(400);

impl AppState {
    /// Puts the selection on the clipboard; `cut` makes the paste a move.
    pub fn copy_selection(self: &Rc<Self>, cut: bool) {
        let paths = self.selection();
        if paths.is_empty() {
            return;
        }
        let files: Vec<gio::File> = paths.iter().map(gio::File::for_path).collect();
        let uris: Vec<String> = files.iter().map(|f| f.uri().to_string()).collect();
        let gnome = format!("{}\n{}", if cut { "cut" } else { "copy" }, uris.join("\n"));
        let text: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        let content = gdk::ContentProvider::new_union(&[
            gdk::ContentProvider::for_bytes(GNOME_COPIED, &glib::Bytes::from_owned(gnome)),
            gdk::ContentProvider::for_value(&gdk::FileList::from_array(&files).to_value()),
            gdk::ContentProvider::for_value(&text.join("\n").to_value()),
        ]);
        if let Err(e) = self.window.clipboard().set_content(Some(&content)) {
            self.toast(&format!("Couldn’t use the clipboard: {}", e));
            return;
        }
        let what = items(paths.len());
        self.toast(&if cut {
            format!("Cut {}: paste to move", what)
        } else {
            format!("Copied {}", what)
        });
    }

    /// Pastes files from the clipboard into the current folder.
    pub fn paste(self: &Rc<Self>) {
        let clipboard = self.window.clipboard();
        let state = self.clone();
        glib::spawn_future_local(async move {
            let (paths, mode) = match read_clipboard(&clipboard).await {
                Some(found) if !found.0.is_empty() => found,
                _ => {
                    state.toast("There are no files on the clipboard");
                    return;
                }
            };
            let dest = state.current_path();
            state.transfer(paths, dest, mode).await;
            if mode == Mode::Move {
                // The originals are gone; a second paste would fail.
                clipboard.set_content(None::<&gdk::ContentProvider>).ok();
            }
        });
    }

    /// Files dropped on the view (`dest` = the folder dropped on).
    pub fn drop_files(
        self: &Rc<Self>,
        paths: Vec<PathBuf>,
        dest: PathBuf,
        modifiers: gdk::ModifierType,
    ) {
        // Dropping an item on itself (or where it already is) is a no-op.
        let paths: Vec<PathBuf> = paths.into_iter().filter(|p| *p != dest).collect();
        let Some(first) = paths.first() else { return };
        let mode = if modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
            Mode::Copy
        } else if modifiers.contains(gdk::ModifierType::SHIFT_MASK) || same_device(first, &dest) {
            Mode::Move
        } else {
            // Like other file managers: another disk means copy.
            Mode::Copy
        };
        let state = self.clone();
        glib::spawn_future_local(async move {
            state.transfer(paths, dest, mode).await;
        });
    }

    /// Runs a copy / move off the main thread and reports the result.
    async fn transfer(self: &Rc<Self>, sources: Vec<PathBuf>, dest: PathBuf, mode: Mode) {
        let n = sources.len();
        let (doing, done_verb) = match mode {
            Mode::Copy => ("Copying", "Copied"),
            Mode::Move => ("Moving", "Moved"),
        };
        let progress = adw::Toast::builder()
            .title(format!("{} {}…", doing, items(n)))
            .timeout(0)
            .build();
        let finished = Rc::new(std::cell::Cell::new(false));
        {
            let (toasts, progress, finished) = (
                self.chrome.toasts.clone(),
                progress.clone(),
                finished.clone(),
            );
            glib::timeout_add_local_once(PROGRESS_DELAY, move || {
                if !finished.get() {
                    toasts.add_toast(progress);
                }
            });
        }

        let job_dest = dest.clone();
        let results = gio::spawn_blocking(move || transfer::transfer(&sources, &job_dest, mode))
            .await
            .unwrap_or_default();
        finished.set(true);
        progress.dismiss();

        let created: Vec<PathBuf> = results
            .iter()
            .filter_map(|(_, o)| match o {
                Outcome::Done(p) => Some(p.clone()),
                _ => None,
            })
            .collect();
        let failures: Vec<String> = results
            .iter()
            .filter_map(|(src, o)| match o {
                Outcome::Failed(e) => Some(format!("“{}”: {}", file_name(src), e)),
                _ => None,
            })
            .collect();

        if dest == self.current_path() {
            // Select what arrived once the file view sees it.
            self.preselect(created.clone());
        }
        self.after_file_op();

        match failures.as_slice() {
            [] if dest != self.current_path() && !created.is_empty() => {
                self.toast(&format!(
                    "{} {} to “{}”",
                    done_verb,
                    items(created.len()),
                    file_name(&dest)
                ));
            }
            [] => {}
            [one] => {
                self.toast(&format!("Couldn’t {} {}", verb(mode), one));
            }
            [first, rest @ ..] => {
                self.toast(&format!(
                    "Couldn’t {} {} (+{} more)",
                    verb(mode),
                    first,
                    rest.len()
                ));
            }
        }
    }
}

/// The clipboard's files and whether they were cut, in the richest format
/// on offer.
async fn read_clipboard(clipboard: &gdk::Clipboard) -> Option<(Vec<PathBuf>, Mode)> {
    if clipboard.formats().contain_mime_type(GNOME_COPIED) {
        if let Ok((stream, _)) = clipboard
            .read_future(&[GNOME_COPIED], glib::Priority::DEFAULT)
            .await
        {
            let mut data = vec![];
            while let Ok(chunk) = stream
                .read_bytes_future(64 * 1024, glib::Priority::DEFAULT)
                .await
            {
                if chunk.is_empty() {
                    break;
                }
                data.extend_from_slice(&chunk);
            }
            if let Some(found) = parse_gnome_copied(&String::from_utf8_lossy(&data)) {
                return Some(found);
            }
        }
    }
    let value = clipboard
        .read_value_future(gdk::FileList::static_type(), glib::Priority::DEFAULT)
        .await
        .ok()?;
    let files = value.get::<gdk::FileList>().ok()?.files();
    Some((files.iter().filter_map(|f| f.path()).collect(), Mode::Copy))
}

/// "copy\nfile:///a\nfile:///b" → paths and mode. Non-local URIs are skipped.
fn parse_gnome_copied(src: &str) -> Option<(Vec<PathBuf>, Mode)> {
    let mut lines = src.lines().map(str::trim).filter(|l| !l.is_empty());
    let mode = match lines.next()? {
        "copy" => Mode::Copy,
        "cut" => Mode::Move,
        _ => return None,
    };
    let paths = lines
        .filter(|l| l.starts_with("file://"))
        .filter_map(|uri| gio::File::for_uri(uri).path())
        .collect();
    Some((paths, mode))
}

fn same_device(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::symlink_metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev(),
        _ => false,
    }
}

fn verb(mode: Mode) -> &'static str {
    match mode {
        Mode::Copy => "copy",
        Mode::Move => "move",
    }
}

fn items(n: usize) -> String {
    if n == 1 {
        "1 item".into()
    } else {
        format!("{} items", n)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nautilus_clipboard() {
        let (paths, mode) =
            parse_gnome_copied("cut\nfile:///home/u/a%20b.txt\nsftp://x/y\nfile:///tmp\n").unwrap();
        assert_eq!(mode, Mode::Move);
        assert_eq!(
            paths,
            [PathBuf::from("/home/u/a b.txt"), PathBuf::from("/tmp")]
        );
        assert_eq!(parse_gnome_copied("copy\n").unwrap(), (vec![], Mode::Copy));
        assert!(parse_gnome_copied("hello\nfile:///x").is_none());
    }
}
