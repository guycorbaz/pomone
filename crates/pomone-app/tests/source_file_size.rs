//! Workspace-wide source-size gate (issue #116).
//!
//! Project rule (2026-07-12): no `.rs` source file may exceed 3000 lines,
//! and 2000 is the working target. Oversized files must be split into
//! modules (see `pomone-ui/src/wiring/`) or extracted libraries. Files
//! between the target and the hard cap are listed as warnings so the
//! trend is visible before the gate trips.

use std::fs;
use std::path::{Path, PathBuf};

/// Hard cap: the test fails when any non-exempt file crosses this.
const MAX_LINES: usize = 3000;
/// Working target: files above this are reported but do not fail.
const TARGET_LINES: usize = 2000;

/// Known oversized files, each carried as documented debt with its exit
/// plan. Keep this list shrinking — never grow it without an issue.
const EXEMPT: &[&str] = &[];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/crates/pomone-app
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root above crates/pomone-app")
        .to_path_buf()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable source directory") {
        let path = entry.expect("readable directory entry").path();
        if path.is_dir() {
            // `target/` never appears under `crates/*/src|tests`, but stay
            // defensive in case build artifacts ever land here.
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_source_file_exceeds_the_line_cap() {
    let root = workspace_root();
    let crates_dir = root.join("crates");
    assert!(
        crates_dir.is_dir(),
        "crates/ not found under {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_rs_files(&crates_dir, &mut files);
    assert!(!files.is_empty(), "no .rs files found — walker broken?");

    let mut over_cap = Vec::new();
    let mut over_target = Vec::new();
    for path in files {
        let lines = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
            .lines()
            .count();
        let rel = path
            .strip_prefix(&root)
            .expect("path under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        let exempt = EXEMPT.contains(&rel.as_str());
        if lines > MAX_LINES && !exempt {
            over_cap.push(format!("{rel} ({lines} lines)"));
        } else if lines > TARGET_LINES && !exempt {
            over_target.push(format!("{rel} ({lines} lines)"));
        }
    }

    if !over_target.is_empty() {
        println!(
            "warning: files above the {TARGET_LINES}-line target (split before they \
             reach {MAX_LINES}):\n  {}",
            over_target.join("\n  ")
        );
    }
    assert!(
        over_cap.is_empty(),
        "source files exceed the {MAX_LINES}-line cap — split them into modules \
         (see pomone-ui/src/wiring/) or add a justified, issue-tracked exemption:\n  {}",
        over_cap.join("\n  ")
    );
}
