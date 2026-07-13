//! Lint gate (story 1.2): the settled-state task columns
//! (`completed_on`, `skipped_on`, `skip_reason`, `skip_note`) may be written by
//! `UPDATE` **only** inside a `facts.rs` file — `facts::record_fact` is the
//! single write path. Any other `UPDATE task SET <settled column>` is a bug.
//!
//! This scans the workspace source so it also catches a stray projection added
//! later in a view/service/UI file.

use std::fs;
use std::path::{Path, PathBuf};

/// Settled-state columns that only `facts.rs` may assign in an UPDATE.
const SETTLED_COLUMNS: &[&str] = &["completed_on", "skipped_on", "skip_reason", "skip_note"];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/crates/pomone-db
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root above crates/pomone-db")
        .to_path_buf()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable directory") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Normalise source for robust matching: collapse every whitespace run to a
/// single space, drop spaces around `=`, and lowercase. So `UPDATE  task  SET`,
/// `completed_on=?` and `completed_on = ?` all reduce to the same shape and
/// can't dodge the gate on formatting alone.
fn normalize(src: &str) -> String {
    src.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" = ", "=")
        .replace(" =", "=")
        .replace("= ", "=")
        .to_lowercase()
}

/// Every `update task set …` statement window in the normalised source (from
/// the marker up to the terminating `where`, or 400 chars — enough for our SQL).
fn update_task_windows(normalized: &str) -> Vec<&str> {
    let marker = "update task set";
    let mut windows = Vec::new();
    let mut from = 0;
    while let Some(rel) = normalized[from..].find(marker) {
        let start = from + rel;
        let rest = &normalized[start..];
        let end = rest.find("where").unwrap_or(rest.len().min(400));
        windows.push(&rest[..end]);
        from = start + marker.len();
    }
    windows
}

#[test]
fn settled_columns_are_written_only_in_facts_files() {
    let root = workspace_root();
    let crates_dir = root.join("crates");
    let mut files = Vec::new();
    collect_rs_files(&crates_dir, &mut files);
    assert!(!files.is_empty(), "no source files found — walker broken?");

    let mut offenders: Vec<String> = Vec::new();
    for path in &files {
        let is_facts = path.file_name().is_some_and(|n| n == "facts.rs");
        // The lint test itself names the columns in strings — skip it.
        if path.file_name().is_some_and(|n| n == "facts_write_path.rs") {
            continue;
        }
        let normalized = normalize(&fs::read_to_string(path).expect("readable source"));
        for window in update_task_windows(&normalized) {
            for col in SETTLED_COLUMNS {
                if window.contains(&format!("{col}=")) && !is_facts {
                    let rel = path.strip_prefix(&root).unwrap_or(path);
                    offenders.push(format!("{} — UPDATE task SET … {col} =", rel.display()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "settled task-state columns must be projected only by facts::record_fact \
         (a `facts.rs` file). Offending writes:\n  {}",
        offenders.join("\n  ")
    );
}
