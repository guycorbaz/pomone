---
project_name: 'pomone'
user_name: 'Guy'
date: '2026-07-07'
sections_completed:
  ['project_origin', 'technology_stack', 'architecture_rules', 'backend_migration_rules', 'error_i18n_rules', 'ui_rules', 'testing_rules', 'quality_rules', 'workflow_rules', 'anti_patterns']
status: 'complete'
rule_count: 34
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Project Origin & Reference

Pomone is a **Rust rewrite of Qrop** (https://qrop.readthedocs.io/), the C++/Qt crop-planning tool for market gardening — reimplemented from scratch with improvements, **not a mechanical port**.

- **Key improvement over Qrop:** Pomone models **annual AND pluriannual (perennial)** crops from the ground up (`Lifespan::Pluriannual`, `VarietyProfile`, `PruningSeason`). Qrop is annual-only — do not assume a feature exists in Pomone just because it does in Qrop, and vice-versa.
- **The Qrop source is checked out locally at `../qrop-main`** (relative to the repo root). For any behaviour/domain question — planting-date maths, task generation, harvest windows — **consult `../qrop-main` rather than guessing.** It is the reference for domain semantics.
- Treat divergences as deliberate: where Pomone differs from Qrop, the perennial model or a stated improvement is usually the reason.

## Technology Stack & Versions

- **Rust** edition 2021, MSRV **1.80**. Cargo workspace, `resolver = "2"`, GPL-3.0.
- **5 crates, strict dependency stack:** `pomone-domain ← pomone-db ← pomone-app ← pomone-ui / pomone-cli`. A layer only depends on those below it — never upward, never sideways.
- **Persistence:** sqlx 0.8 (`runtime-tokio-rustls`; `sqlite`, `mysql`, `chrono`, `uuid`, `rust_decimal`, `migrate`, `macros`).
- **Domain types:** chrono 0.4 (`NaiveDate`), rust_decimal 1.36, uuid 1.11 (v4).
- **UI:** Slint 1.8, `renderer-software` (pure-Rust; no skia/fontconfig runtime dep). The clippy/test build still needs `libfontconfig-dev pkg-config` for Slint's native build.
- **i18n:** fluent 0.16 — catalogues at `crates/pomone-app/locales/{fr,en}/main.ftl`.
- **Async:** tokio 1.42. **CLI:** clap 4. **Errors:** thiserror 2 / anyhow 1.
- **Tests:** proptest, rstest, insta, pretty_assertions, testcontainers (MariaDB).

## Critical Implementation Rules

### Architecture & Layering

- `pomone-domain` is **pure — no I/O.** All invariants live in constructors (`Variety::new`, `Crop::new`, `Lifespan::perennial`, …) returning `DomainResult`. Never validate domain rules elsewhere.
- A variety's `VarietyProfile` must match its crop's `Lifespan` — enforced by `check_compatible`. When constructing/editing a variety, go through the constructor so this check runs; never assemble the struct by hand.
- Application code depends only on **`&dyn Repository`**, never a concrete backend. Sub-traits (`CropRepo`, `VarietyRepo`, …) aggregate into `Repository`.
- `*_view.rs` modules are **presentation helpers**: take `&dyn Repository`, return plain-string DTOs, parse UI strings back. The UI **never** sees `Uuid` / `Decimal` / domain enums.
- Cross-entity operations live in `services.rs`; task/milestone derivation in `task_autogen.rs`.
- ID parsing is centralised in `plantings_view::parse_id` — reuse it (`crate::plantings_view::parse_id`), don't reimplement.

### Domain: dates & quantities

- **Date logic lives in `pomone-domain` (`date_calc.rs`), NOT in SQL.** These pure functions deliberately **replace Qrop's SQL triggers** so the two backends stay identical and the maths is property-tested. Never add a trigger or date computation in a migration.
- Use the `date_calc` helpers (`add_days`, `date_from_doy`, …) which return `DomainError` on overflow / invalid day-of-year. **Never `.unwrap()` chrono arithmetic** — year boundaries and leap years (day 366) are real inputs.
- Agronomic dates are `chrono::NaiveDate` — **no timezones**.
- Quantities (`area_m2`, `*_yield_kg`, `labor_hours`) are **`rust_decimal::Decimal`, never `f64`.**

### Dual-Backend & Migrations

- `SqliteRepository` and `MariaDbRepository` must stay **behaviourally identical** — `cross_backend_tests.rs` asserts this. Any repo change updates **both** impls + adds cross-backend coverage.
- `codec.rs` centralises encode/decode of domain sum types (`Lifespan`, `VarietyProfile`, `PlantingSchedule`, `PruningSeason`, …) to/from SQL columns. Extend it there — don't scatter conversions.
- **`encode_*` and `decode_*` are a pair.** Adding an enum variant means updating both, keeping both matches exhaustive, and using the **same string literal** on both backends. A missed `decode` arm fails at **runtime** (`DbError::Malformed`), not at compile time.
- A schema change = a **new numbered `.sql` in BOTH `migrations/sqlite/` and `migrations/mariadb/`** (next is `0005_*.sql`) + codec + both impls + tests.
- **Additive migrations only** (`ADD COLUMN` / `ALTER`). SQLite `CHECK` constraints can't gain allowed values without a dangerous table rebuild — extend behaviour via additive seed defaults instead.

### Error Handling & i18n

- Errors are **structured enum variants** (`DomainError`, `DbError`, `AppError`) — never stringly-typed for user display.
- The UI maps errors to Fluent keys via `localize_app_error` / `localize_domain_error` in `pomone-ui/src/main.rs`. FK-violation guards surface as `AppError::Inconsistent("<sentinel>")`, re-keyed to a localized `error-*` string.
- **Every user-facing string needs BOTH `fr` and `en` keys.** `fr/main.ftl` and `en/main.ftl` are mirrors — same key set — with keys **alphabetical within each section**. Interpolation uses `{ $name }`.

### Slint UI Plumbing (3 layers)

- A property/callback is wired in three files: the `*Page` component (`ui/<page>.slint`) declares `in`/`in-out` props + `callback`s → `main.slint` re-declares them on `MainWindow` and forwards to the page → `main.rs` uses generated `get_*`/`set_*`/`on_*`.
- **Adding a property means touching all three files** or the generated method won't exist.

### Editing Pattern (fixed shape across catalogs)

- Crop / variety / location editing follows one shape: a `get_*_for_edit` DTO getter + an `update_*` service that **rebuilds + validates via the domain constructor and keeps the original id**, mirrored in the UI as an edit-mode form. **Copy the nearest sibling — don't invent a new pattern.**

### Testing Rules

- MariaDB backend tests are `#[ignore = "requires Docker for MariaDB testcontainer"]` — run with `cargo test -- --ignored`. Coverage counts the MariaDB backend at 0% by design.
- CI enforces an **80% coverage gate** (`cargo llvm-cov --workspace`) — new code carries its tests or the build goes red.
- Run isolated against a temp DB by overriding XDG dirs: `XDG_DATA_HOME=/tmp/pom XDG_CONFIG_HOME=/tmp/pom cargo run -p pomone-ui`.
- No `sqlite3` CLI on the dev machine — inspect DBs with `python3`'s `sqlite3` module.

### Code Quality, Lints & CI

- CI runs with **`RUSTFLAGS: -D warnings`** — **any warning fails the build.** Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` before pushing.
- `unsafe_code = "deny"`, `unused_must_use = "deny"`, clippy `pedantic`. The pragmatic allow-list lives in root `Cargo.toml` `[workspace.lints]` — rely on it, don't sprinkle local `#[allow]`.
- CI is **Linux-only** this phase (no macOS/Windows legs).

### Development Workflow Rules

- **Never push to `main`** (protected). Work on a branch, open a PR (`gh pr create`), CI must be green.

### Critical Don't-Miss Rules — Adding a persisted field (full blast radius)

Adding one persisted field to a domain type ripples through the whole stack. Miss a link and you get a compile error (Slint / exhaustive codec) or — worse — a **silent backend divergence** (codec / row mapper). Checklist:

```
domain      → constructor + validation
codec       → encode + decode (exhaustive, identical string on both backends)
migration   → NNNN_*.sql in sqlite AND mariadb (additive only)
db backends → row mappers in BOTH SqliteRepository and MariaDbRepository
app/view    → DTO field (String) + parser
ui (Slint)  → page.slint + main.slint + main.rs  (all three)
i18n        → label key in fr/main.ftl AND en/main.ftl
tests       → cross_backend_tests coverage
```

---

## Usage Guidelines

**For AI Agents:**

- Read this file before implementing any code.
- Follow ALL rules exactly. When in doubt, prefer the more restrictive option and consult `../qrop-main` for domain semantics.
- The single most common failure is doing half of a cross-layer change — always walk the "Adding a persisted field" checklist for schema/domain changes.

**For Humans:**

- Keep this file lean and focused on agent needs.
- Update when the technology stack or a core pattern changes.
- Review periodically; remove rules that become obvious over time.

Last Updated: 2026-07-07
