//! Glossary ↔ Fluent coherence gate (story 0.6).
//!
//! Asserts that the founding glossary (`docs/glossaire.md`) and the Fluent
//! catalogues (`locales/{fr,en}/main.ftl`) agree: every glossary term marked
//! `checked` must resolve to at least one Fluent key under its declared prefix,
//! **in both locales** (no orphan term, no half-translated term).
//!
//! The check is **scoped to `checked` rows** so it is born green: founding
//! terms whose Fluent alignment is still planned (renames in story 0.8, or
//! concepts introduced by a later epic) are listed as `deferred` and skipped
//! until their scope is flipped. Story 0.8 widens the scope to every term.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/crates/pomone-app
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root above crates/pomone-app")
        .to_path_buf()
}

/// Every top-level Fluent message key in a `.ftl` file. A key line looks like
/// `some-key = value`; comments (`#`) and indented continuation/attribute lines
/// are ignored.
fn ftl_keys(path: &Path) -> BTreeSet<String> {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut keys = BTreeSet::new();
    for line in text.lines() {
        // Key lines start in column 0 with an identifier; skip comments,
        // blank lines and indented attribute/continuation lines.
        if line.is_empty() || line.starts_with('#') || line.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((lhs, _)) = line.split_once('=') else {
            continue;
        };
        let key = lhs.trim();
        if !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            keys.insert(key.to_string());
        }
    }
    keys
}

/// A key belongs to a term's prefix when it *is* the prefix or sits under it on
/// a hyphen boundary — so `category` matches `category-sow` but not `categoryx`,
/// and `crop` matches `crop` and `crop-map-title` but not `crops-title`.
fn matches_prefix(key: &str, prefix: &str) -> bool {
    key == prefix || key.starts_with(&format!("{prefix}-"))
}

struct GlossaryRow {
    term_id: String,
    prefixes: Vec<String>,
    scope: String,
}

/// Split a Markdown table row into trimmed cells (outer pipes stripped).
fn table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
}

/// Parse the founding-terms table, mapping columns by header name so the test
/// survives column reordering.
fn parse_glossary(path: &Path) -> Vec<GlossaryRow> {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    let mut header: Option<(usize, usize, usize)> = None; // (term_id, prefix, scope) column indices
    let mut rows = Vec::new();

    for line in text.lines() {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let cells = table_cells(line);
        if is_separator_row(&cells) {
            continue;
        }
        match header {
            None => {
                // First table row is the header: locate our three columns.
                let find =
                    |needle: &str| cells.iter().position(|c| c.to_lowercase().contains(needle));
                let term = find("term_id");
                let prefix = find("fluent");
                let scope = find("portée")
                    .or_else(|| find("scope"))
                    .or_else(|| find("ci"));
                if let (Some(t), Some(p), Some(s)) = (term, prefix, scope) {
                    header = Some((t, p, s));
                }
            }
            Some((t, p, s)) => {
                let max = t.max(p).max(s);
                if cells.len() <= max {
                    continue; // not a data row of our table
                }
                let prefixes = cells[p]
                    .split([' ', ','])
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "—")
                    .map(str::to_string)
                    .collect();
                rows.push(GlossaryRow {
                    term_id: cells[t].clone(),
                    prefixes,
                    scope: cells[s].to_lowercase(),
                });
            }
        }
    }

    assert!(
        header.is_some(),
        "glossary table header (term_id / Fluent prefix / CI scope) not found in {}",
        path.display()
    );
    rows
}

#[test]
fn glossary_terms_resolve_to_fluent_keys_in_both_locales() {
    let root = workspace_root();
    let glossary = root.join("docs/glossaire.md");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fr_keys = ftl_keys(&manifest.join("locales/fr/main.ftl"));
    let en_keys = ftl_keys(&manifest.join("locales/en/main.ftl"));

    let rows = parse_glossary(&glossary);
    assert!(
        !rows.is_empty(),
        "no glossary rows parsed from {}",
        glossary.display()
    );

    // Guard against typos in the scope column silently disabling checks.
    for row in &rows {
        assert!(
            matches!(row.scope.as_str(), "checked" | "deferred"),
            "term '{}' has unknown CI scope '{}' (expected 'checked' or 'deferred')",
            row.term_id,
            row.scope
        );
    }

    let checked: Vec<&GlossaryRow> = rows.iter().filter(|r| r.scope == "checked").collect();
    assert!(
        !checked.is_empty(),
        "no 'checked' glossary terms — the coherence gate would be vacuous"
    );

    let mut term_ids = BTreeSet::new();
    let mut problems: Vec<String> = Vec::new();
    for row in &rows {
        if !term_ids.insert(row.term_id.clone()) {
            problems.push(format!("duplicate term_id '{}'", row.term_id));
        }
    }

    for row in &checked {
        if row.prefixes.is_empty() {
            problems.push(format!(
                "term '{}' is 'checked' but declares no Fluent prefix",
                row.term_id
            ));
            continue;
        }
        for prefix in &row.prefixes {
            let fr: BTreeSet<&String> = fr_keys
                .iter()
                .filter(|k| matches_prefix(k, prefix))
                .collect();
            let en: BTreeSet<&String> = en_keys
                .iter()
                .filter(|k| matches_prefix(k, prefix))
                .collect();

            if fr.is_empty() && en.is_empty() {
                problems.push(format!(
                    "term '{}': Fluent prefix '{prefix}' matches no key in either locale",
                    row.term_id
                ));
                continue;
            }
            // Translations must not omit the term: same key set both sides.
            let only_fr: Vec<&str> = fr.difference(&en).map(|s| s.as_str()).collect();
            let only_en: Vec<&str> = en.difference(&fr).map(|s| s.as_str()).collect();
            if !only_fr.is_empty() {
                problems.push(format!(
                    "term '{}': prefix '{prefix}' keys present in fr but missing in en: {only_fr:?}",
                    row.term_id
                ));
            }
            if !only_en.is_empty() {
                problems.push(format!(
                    "term '{}': prefix '{prefix}' keys present in en but missing in fr: {only_en:?}",
                    row.term_id
                ));
            }
        }
    }

    assert!(
        problems.is_empty(),
        "glossary ↔ Fluent coherence failures ({} term(s) checked):\n  {}",
        checked.len(),
        problems.join("\n  ")
    );
}
