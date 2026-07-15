# Dogfooding database-compatibility runbook

**Goal.** Prove that the owner's real, accumulating database still migrates to
the current schema and serves the app after every schema change — the concrete
guard behind NFR8 ("decade-old data round-trips all migrations on both
backends"). This is the dogfooding safety net: the base Pomone is developed
*against* must never be stranded by a migration.

This procedure is **local-only and CI-ignored** (real data never enters CI). Run
it **once per schema change** — i.e. whenever a new `migrations/**/NNNN_*.sql`
pair lands.

## What the test does

`crates/pomone-app/tests/dogfood_db.rs` (`dogfood_real_database_migrates_and_smokes`,
`#[ignore]`d):

1. Reads the path in `POMONE_DOGFOOD_DB`.
2. **Copies** that file (plus any `-wal`/`-shm` sidecars) into a temp dir — it
   never opens the source, so the original is untouched.
3. Opens the copy through `App::new`, which runs **every embedded migration** up
   to the current schema. A migration that cannot apply to real historical data
   fails here.
4. Smoke-reads the migrated data back through the repositories and the
   presentation layer (`crop_list` / `planting_list` / `task_list`, then
   `list_agenda` and `build_week_sheet`) — the latter two exercise the Epic-1
   fact-projection columns (`completed_on` / `skipped_on` / `skip_reason`)
   specifically.

If all steps pass, the current schema serves the real data.

## Preparing a sanitized copy

Pomone stores crop-planning data (crops, varieties, locations, plantings, tasks,
treatments, harvests). There are no credentials or personal identifiers in the
schema, so "sanitization" is mainly about **not committing the file** and, if you
want to share a reproduction, redacting free-text notes. Minimum procedure:

```sh
# Work from a copy, never the live file.
cp ~/.local/share/pomone/pomone.sqlite /tmp/pomone-dogfood.sqlite
# (optional) blank free-text notes if the copy will leave your machine:
python3 - <<'PY'
import sqlite3
db = sqlite3.connect("/tmp/pomone-dogfood.sqlite")
for tbl, col in [("task","notes"), ("planting","notes"), ("treatment","notes")]:
    try: db.execute(f"UPDATE {tbl} SET {col} = NULL")
    except sqlite3.OperationalError: pass
db.commit(); db.close()
PY
```

The copy is disposable — never add it to git.

## Running it

```sh
POMONE_DOGFOOD_DB=/tmp/pomone-dogfood.sqlite \
  cargo test -p pomone-app --test dogfood_db -- --ignored --nocapture
```

Expected tail on success:

```
dogfood OK: migrated + smoked — <N> crops, <N> plantings, <N> tasks, <N> agenda rows
```

If `POMONE_DOGFOOD_DB` is unset the test prints a skip notice and passes (opt-in
only). If a migration or a read fails, the test fails with the offending error —
fix the migration (additive-only; see `CLAUDE.md`) before shipping the schema
change.

## When to run

- After adding any `migrations/{sqlite,mariadb}/NNNN_*.sql`.
- Before tagging a release.
- Record the run (date + row counts) in the story/PR that introduced the schema
  change.
