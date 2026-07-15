# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Pomone is a Rust rewrite of [Qrop](https://qrop.readthedocs.io/) (C++/Qt) — a free crop-planning tool for market gardening, field crops, orcharding and agroforestry. Unlike Qrop it models **annual and pluriannual (perennial)** crops from the ground up. Native desktop UI (Slint), data in SQLite **or** MariaDB behind one trait, i18n FR/EN via Fluent. The Qrop reference source is checked out locally at `../qrop-main` — consult it for behaviour/domain questions rather than guessing.

## Commands

```sh
cargo build --release
cargo test --workspace                       # ~340 tests; MariaDB tests are #[ignore]d
cargo test -p pomone-app cultures_view::     # one module in one crate
cargo test -p pomone-app update_variety_changes_name_and_profile   # one test by name
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo llvm-cov --workspace --html            # coverage gate is ≥80% (needs cargo-llvm-cov)

cargo run -p pomone-ui                        # launch the desktop app (`pomone`)
cargo run -p pomone-cli -- seed-demo          # admin/debug CLI: seed-demo | info | backup | restore
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy, test, and the 80% coverage gate — **`RUSTFLAGS: -D warnings`, so any warning fails the build.** CI is **Linux-only** for now (macOS/Windows legs dropped this phase). The clippy/test jobs need `libfontconfig-dev pkg-config` for Slint's native build.

The MariaDB backend tests are marked `#[ignore = "requires Docker for MariaDB testcontainer"]`; they spin up a real MariaDB via `testcontainers` and only run with Docker available (`cargo test -- --ignored`). Coverage counts the MariaDB backend at 0% by design.

### Running against an isolated database

The app writes to the OS data dir (`~/.local/share/pomone/pomone.sqlite` on Linux). To avoid touching real data, override the XDG dirs:

```sh
XDG_DATA_HOME=/tmp/pom XDG_CONFIG_HOME=/tmp/pom cargo run -p pomone-ui
XDG_DATA_HOME=/tmp/pom XDG_CONFIG_HOME=/tmp/pom cargo run -p pomone-cli -- seed-demo
```

There is no `sqlite3` CLI on the dev machine — inspect databases with `python3`'s `sqlite3` module.

## Architecture

Five crates form a strict dependency stack (`domain` ← `db` ← `app` ← `ui`/`cli`); each layer only knows the ones below it.

- **`pomone-domain`** — pure business types + validation, **no I/O**. Constructors (`Variety::new`, `Crop::new`, `Lifespan::perennial`, …) return `DomainResult` and are the single place invariants are enforced. Key sum types: `Lifespan` (Annual / Pluriannual{SingleCycle | Recurring}), `VarietyProfile` (Annual / Pluriannual), `PlantingSchedule`, `PlantingStatus`, `PruningSeason`. A variety's profile must match its crop's lifespan (`check_compatible`).

- **`pomone-db`** — persistence behind the `Repository` trait (`repository.rs`). Each entity has a sub-trait (`CropRepo`, `VarietyRepo`, …) aggregated into `Repository`; **application code depends only on `&dyn Repository`**, never a concrete backend. Two impls — `SqliteRepository` and `MariaDbRepository` (`sqlite/`, `mariadb/`) — must stay behaviourally identical; `cross_backend_tests.rs` asserts that. `codec.rs` centralises encode/decode of the domain sum types to/from SQL columns. Schema lives in `migrations/{sqlite,mariadb}/NNNN_*.sql`, embedded at compile time via `sqlx::migrate!` and run on connect.

- **`pomone-app`** — use cases and view-models. `App` (`app.rs`) owns a `Box<dyn Repository>` + `I18n`; `swap_backend` migrates data live between SQLite and MariaDB. `services.rs` holds cross-entity operations (create plantings, auto-generate tasks, harvests). The `*_view.rs` modules are **presentation helpers**: they take `&dyn Repository`, return plain-string DTOs (the UI never sees `Uuid`/`Decimal`/domain enums), and parse UI strings back. `task_autogen.rs` derives sow/transplant/harvest tasks and milestones from plantings.

- **`pomone-ui`** — the `pomone` binary. Slint markup in `ui/*.slint`; `src/main.rs` is the Rust↔Slint glue that wires every Slint `callback` to an `app` view/service call. **Slint property/callback plumbing is three-layered**: a `*Page` component declares `in`/`in-out` properties + `callback`s, `main.slint` re-declares them on `MainWindow` and forwards to the page instance, and `main.rs` gets/sets them via generated `get_*`/`set_*`/`on_*` methods. Adding a new property to a page means touching all three files or the generated method won't exist.

- **`pomone-cli`** — the `pomone-cli` admin/debug binary (`seed-demo`, `info`, `backup`, `restore`).

### Error handling & i18n

Domain/DB/app errors are **structured enum variants** (`DomainError`, `DbError`, `AppError`), never stringly-typed for user display. The UI maps them to Fluent keys: message catalogues are `crates/pomone-app/locales/{fr,en}/main.ftl`, resolved through `localize_app_error` / `localize_domain_error` in `pomone-ui/src/main.rs`. FK-violation guards (e.g. deleting an in-use crop/variety) surface as an `AppError::Inconsistent("<sentinel>")` that the UI re-keys to a localized `error-*` string. **Any user-facing string must have both `fr` and `en` keys.**

## Conventions

- **Never push to `main`.** It is protected — work on a branch and open a PR (`gh pr create`). CI must be green.
- **Adding a schema change** = a new numbered `.sql` in *both* `migrations/sqlite/` and `migrations/mariadb/`, plus codec + both backend impls + `cross_backend_tests` coverage. Prefer `ADD COLUMN`/`ALTER`-only migrations; SQLite `CHECK` constraints (e.g. on `task_type.category`) can't gain new allowed values without a dangerous table rebuild — extend behaviour via additive seed defaults instead.
- **Editing follows a fixed shape** across catalogs (crop/variety/location): a `get_*_for_edit` DTO getter + an `update_*` service that rebuilds+validates via the domain constructor and keeps the original id, mirrored in the UI as an edit-mode form. Copy the nearest sibling rather than inventing a new pattern.
- Lints are strict (`unsafe_code = deny`, clippy `pedantic`); pragmatic allow-list is in the root `Cargo.toml` `[workspace.lints]`. MSRV is 1.80.
- The technical doc (`doc-latex/`) and user manual (`docs/manual/`) are LaTeX (xelatex); the manual PDF is embedded in the app and opened with `F1`.
