//! Glossary ↔ Fluent coherence gate (stories 0.6 + 0.8).
//!
//! Asserts that the founding glossary (`docs/glossaire.md`, "Table 1") and the
//! Fluent catalogues (`locales/{fr,en}/main.ftl`) agree: **every** founding
//! term must resolve to at least one Fluent key under its declared prefix, in
//! **both locales** (no orphan term, no half-translated term).
//!
//! Since story 0.8 the check is **unscoped**: there is no per-term opt-in — the
//! whole founding table is covered. Concepts not yet wired (or without a stable
//! Fluent anchor) live in the glossary's "Table 2" (planned/documented
//! vocabulary), which this test deliberately does not parse; the epic that
//! wires such a concept promotes it into Table 1 with its keys.

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

struct FoundingTerm {
    term_id: String,
    prefix: String,
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

/// Parse **Table 1** only: the first Markdown table whose header carries both a
/// `term_id` and a `Fluent prefix` column. Reading stops at the end of that
/// table, so Table 2 (planned vocabulary, different columns) is never seen.
fn parse_founding_terms(path: &Path) -> Vec<FoundingTerm> {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    let mut columns: Option<(usize, usize)> = None; // (term_id, prefix)
    let mut terms = Vec::new();

    for line in text.lines() {
        let is_row = line.trim_start().starts_with('|');
        match columns {
            None => {
                if !is_row {
                    continue;
                }
                let cells = table_cells(line);
                let find =
                    |needle: &str| cells.iter().position(|c| c.to_lowercase().contains(needle));
                if let (Some(t), Some(p)) = (find("term_id"), find("fluent")) {
                    columns = Some((t, p));
                }
            }
            Some((t, p)) => {
                // The founding table is a contiguous block; the first
                // non-table line ends it (and shields Table 2 from parsing).
                if !is_row {
                    break;
                }
                let cells = table_cells(line);
                if is_separator_row(&cells) {
                    continue;
                }
                if cells.len() > t.max(p) {
                    terms.push(FoundingTerm {
                        term_id: cells[t].clone(),
                        prefix: cells[p].clone(),
                    });
                }
            }
        }
    }

    assert!(
        columns.is_some(),
        "founding-terms table (term_id + Fluent prefix columns) not found in {}",
        path.display()
    );
    terms
}

#[test]
fn founding_terms_resolve_to_fluent_keys_in_both_locales() {
    let root = workspace_root();
    let glossary = root.join("docs/glossaire.md");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fr_keys = ftl_keys(&manifest.join("locales/fr/main.ftl"));
    let en_keys = ftl_keys(&manifest.join("locales/en/main.ftl"));

    let terms = parse_founding_terms(&glossary);
    assert!(
        !terms.is_empty(),
        "no founding terms parsed from {}",
        glossary.display()
    );

    let mut seen = BTreeSet::new();
    let mut problems: Vec<String> = Vec::new();

    for term in &terms {
        if !seen.insert(term.term_id.clone()) {
            problems.push(format!("duplicate term_id '{}'", term.term_id));
        }
        if term.prefix.is_empty() || term.prefix == "—" {
            problems.push(format!(
                "founding term '{}' declares no Fluent prefix",
                term.term_id
            ));
            continue;
        }

        let fr: BTreeSet<&String> = fr_keys
            .iter()
            .filter(|k| matches_prefix(k, &term.prefix))
            .collect();
        let en: BTreeSet<&String> = en_keys
            .iter()
            .filter(|k| matches_prefix(k, &term.prefix))
            .collect();

        if fr.is_empty() && en.is_empty() {
            problems.push(format!(
                "term '{}': Fluent prefix '{}' matches no key in either locale",
                term.term_id, term.prefix
            ));
            continue;
        }
        let only_fr: Vec<&str> = fr.difference(&en).map(|s| s.as_str()).collect();
        let only_en: Vec<&str> = en.difference(&fr).map(|s| s.as_str()).collect();
        if !only_fr.is_empty() {
            problems.push(format!(
                "term '{}': prefix '{}' keys present in fr but missing in en: {only_fr:?}",
                term.term_id, term.prefix
            ));
        }
        if !only_en.is_empty() {
            problems.push(format!(
                "term '{}': prefix '{}' keys present in en but missing in fr: {only_en:?}",
                term.term_id, term.prefix
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "glossary ↔ Fluent coherence failures ({} founding term(s) checked):\n  {}",
        terms.len(),
        problems.join("\n  ")
    );
}
