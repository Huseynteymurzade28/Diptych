use std::path::Path;
use toml_edit::{DocumentMut, Item, Table};

// ═══════════════════════════════════════════════
//  Editing config files in place
// ═══════════════════════════════════════════════
//
// The Customize dialog writes to the same files people edit by hand, so
// edits go through `toml_edit`: comments, ordering and untouched keys stay
// as they were. A change is only written if the result still loads, so the
// dialog can never leave a file the app would reject.

/// Applies `change` to the TOML file at `path` (missing = empty) and writes
/// it back, provided `check` accepts the result.
pub fn update<T>(
    path: &Path,
    check: impl FnOnce(&str) -> Result<T, String>,
    change: impl FnOnce(&mut DocumentMut) -> Result<(), String>,
) -> Result<(), String> {
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let out = edit_str(&src, change)
        .map_err(|e| format!("{} can’t be edited until it’s fixed: {}", name, e))?;
    check(&out).map_err(|e| format!("{}: {}", name, e))?;
    if out == src {
        return Ok(());
    }
    write_atomic(path, &out).map_err(|e| format!("{}: {}", name, e))
}

/// `src` with `change` applied.
pub fn edit_str(
    src: &str,
    change: impl FnOnce(&mut DocumentMut) -> Result<(), String>,
) -> Result<String, String> {
    let mut doc: DocumentMut = src.parse().map_err(|e| first_line(&format!("{}", e)))?;
    change(&mut doc)?;
    Ok(doc.to_string())
}

/// Writes through a temporary file, so a watcher (or a crash) never sees
/// a half-written file.
fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// The table `[name]`, created (at the end of the file) if missing.
pub fn table<'a>(doc: &'a mut DocumentMut, name: &str) -> Result<&'a mut Table, String> {
    if !doc.contains_key(name) {
        let mut t = Table::new();
        t.decor_mut().set_prefix(take_trailing(doc));
        doc.insert(name, Item::Table(t));
    }
    doc[name]
        .as_table_mut()
        .ok_or_else(|| format!("“{}” should be a [{}] table", name, name))
}

/// A table for appending to a document: it takes over the comments at the
/// end of the file, so it lands *below* them (starter files end in
/// commented-out examples) rather than above.
pub fn new_table(doc: &mut DocumentMut) -> Table {
    let mut t = Table::new();
    t.decor_mut().set_prefix(take_trailing(doc));
    t
}

fn take_trailing(doc: &mut DocumentMut) -> String {
    let trailing = doc.trailing().as_str().unwrap_or("").to_string();
    doc.set_trailing("");
    if trailing.trim().is_empty() {
        return "\n".into();
    }
    let mut prefix = trailing.trim_end().to_string();
    prefix.push_str("\n\n");
    prefix
}

/// Sets `[table] key = value`, or the top-level `key` when `table` is `None`.
pub fn set(
    doc: &mut DocumentMut,
    table_name: Option<&str>,
    key: &str,
    value: impl Into<toml_edit::Value>,
) -> Result<(), String> {
    let value = toml_edit::value(value);
    match table_name {
        None => doc[key] = value,
        Some(t) => {
            let t = table(doc, t)?;
            // Keep an existing key's comment and position.
            match t.get_mut(key) {
                Some(item) => keep_decor(item, value),
                None => {
                    t.insert(key, value);
                }
            }
        }
    }
    Ok(())
}

/// Removes `[table] key` (or a top-level key); an emptied table goes too.
pub fn remove(doc: &mut DocumentMut, table_name: Option<&str>, key: &str) {
    match table_name {
        None => remove_item(doc, key),
        Some(t) => {
            let Some(table) = doc.get_mut(t).and_then(Item::as_table_mut) else {
                return;
            };
            table.remove(key);
            if table.is_empty() {
                remove_item(doc, t);
            }
        }
    }
}

/// Removes a top-level key or table. Comments written above it are kept
/// (they move to whatever follows), since in starter files they document
/// other things too.
pub fn remove_item(doc: &mut DocumentMut, key: &str) {
    let order: Vec<String> = doc.iter().map(|(k, _)| k.to_string()).collect();
    let (prefix, position) = match doc.get(key) {
        None => return,
        Some(Item::Table(t)) => (raw(t.decor().prefix()), Some(t.position())),
        Some(Item::ArrayOfTables(_)) => (None, None),
        Some(_) => (
            raw(doc
                .as_table()
                .key(key)
                .and_then(|k| k.leaf_decor().prefix())),
            None,
        ),
    };
    doc.remove(key);
    let Some(prefix) = prefix.filter(|p| p.contains('#')) else {
        return;
    };

    // What followed it: the next plain key (for a key), else the next table.
    let next_value = position.is_none().then(|| {
        order
            .iter()
            .skip_while(|k| *k != key)
            .skip(1)
            .find(|k| doc.get(k).is_some_and(Item::is_value))
            .cloned()
    });
    if let Some(Some(next)) = next_value {
        if let Some(mut k) = doc.as_table_mut().key_mut(&next) {
            let old = raw(k.leaf_decor().prefix()).unwrap_or_default();
            k.leaf_decor_mut().set_prefix(format!("{}{}", prefix, old));
        }
        return;
    }
    let after = position.flatten().unwrap_or(0);
    let next_table = doc
        .iter()
        .filter_map(|(k, item)| Some((k.to_string(), item.as_table()?.position()?)))
        .filter(|(_, p)| position.is_none() || *p > after)
        .min_by_key(|(_, p)| *p)
        .map(|(k, _)| k);
    match next_table.and_then(|k| doc.get_mut(&k)?.as_table_mut()) {
        Some(t) => {
            let old = raw(t.decor().prefix()).unwrap_or_default();
            t.decor_mut().set_prefix(format!("{}{}", prefix, old));
        }
        None => {
            let old = doc.trailing().as_str().unwrap_or("").to_string();
            doc.set_trailing(format!("{}{}", prefix, old));
        }
    }
}

fn raw(r: Option<&toml_edit::RawString>) -> Option<String> {
    r.and_then(|r| r.as_str()).map(str::to_owned)
}

/// Replaces `item`'s value but keeps its surrounding whitespace/comments.
fn keep_decor(item: &mut Item, new: Item) {
    let decor = item.as_value().map(|v| v.decor().clone());
    *item = new;
    if let (Some(decor), Some(v)) = (decor, item.as_value_mut()) {
        *v.decor_mut() = decor;
    }
}

/// A string array value: `["a", "b"]`.
pub fn string_array<S: AsRef<str>>(items: &[S]) -> toml_edit::Value {
    let mut array = toml_edit::Array::new();
    for s in items {
        array.push(s.as_ref());
    }
    toml_edit::Value::Array(array)
}

fn first_line(s: &str) -> String {
    let lines: Vec<&str> = s.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    match (lines.first(), lines.last()) {
        (Some(first), Some(last)) if lines.len() > 1 => format!("{} — {}", first, last),
        (Some(first), _) => first.to_string(),
        _ => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_keeps_comments_and_inline_comments() {
        let src = "# top\nbase = \"nord\"\n\n[shape]\nradius = 14 # corners\n";
        let out = edit_str(src, |d| {
            set(d, Some("shape"), "radius", 6)?;
            set(d, Some("shape"), "density", "compact")?;
            set(d, Some("colors"), "accent", "#ff0000")
        })
        .unwrap();
        assert!(out.starts_with("# top\nbase = \"nord\""), "{}", out);
        assert!(out.contains("radius = 6 # corners"), "{}", out);
        assert!(out.contains("density = \"compact\""));
        assert!(out.contains("[colors]\naccent = \"#ff0000\""), "{}", out);
    }

    #[test]
    fn remove_drops_emptied_tables() {
        let src = "base = \"nord\"\n\n[colors]\naccent = \"#fff\"\n";
        let out = edit_str(src, |d| {
            remove(d, Some("colors"), "accent");
            remove(d, Some("missing"), "x");
            Ok(())
        })
        .unwrap();
        assert_eq!(out.trim(), "base = \"nord\"");
    }

    #[test]
    fn removing_keeps_comments_above() {
        let src = "# about base\nbase = \"nord\"\n# about dark\ndark = true\n# more docs\n# name = \"x\"\n\n# colors doc\n[colors]\naccent = \"#fff\"\n\n# shape doc\n[shape]\nradius = 3\n\n# trailing doc\n";
        let out = edit_str(src, |d| {
            remove_item(d, "dark");
            remove_item(d, "colors");
            remove(d, Some("shape"), "radius");
            Ok(())
        })
        .unwrap();
        for comment in [
            "# about base",
            "# about dark",
            "# more docs",
            "# colors doc",
            "# shape doc",
            "# trailing doc",
        ] {
            assert!(out.contains(comment), "lost {:?}:\n{}", comment, out);
        }
        assert!(
            !out.contains("dark = true") && !out.contains("[colors]") && !out.contains("[shape]"),
            "{}",
            out
        );
        assert!(out.find("# about dark").unwrap() < out.find("# colors doc").unwrap());
    }

    #[test]
    fn refuses_broken_files() {
        let err = edit_str("base = ", |_| Ok(())).unwrap_err();
        assert!(!err.is_empty());
        let err = edit_str("shape = 3", |d| set(d, Some("shape"), "radius", 1)).unwrap_err();
        assert!(err.contains("[shape]"));
    }

    #[test]
    fn update_writes_only_valid_results() {
        let dir = std::env::temp_dir().join(format!("diptych-edit-{}", std::process::id()));
        let path = dir.join("x.toml");
        let _ = std::fs::remove_dir_all(&dir);

        update(&path, |_| Ok(()), |d| set(d, None, "a", 1)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 1\n");

        let err = update(
            &path,
            |s| {
                if s.contains("2") {
                    Err("no".into())
                } else {
                    Ok(())
                }
            },
            |d| set(d, None, "a", 2),
        )
        .unwrap_err();
        assert_eq!(err, "x.toml: no");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 1\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
