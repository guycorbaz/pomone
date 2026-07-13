# Story 0.5: Request structs in services

Status: done

## Story

As a contributor,
I want `services.rs` creation functions to take request structs instead of 8–10 positional parameters,
So that E1/E2 can add fields without breaking every call site twice.

## Acceptance Criteria

1. **Given** the positional signatures **when** request structs are introduced and all call sites migrated (UI, CLI, demo, tests) **then** behavior is unchanged.
2. **And** no creation function exceeds 3 parameters (repo, request, injected date/clock).
3. **And** the `#[allow(clippy::too_many_arguments)]` attributes on the migrated services are removed (the lint no longer fires).
4. **No behavior change** — pure signature/call-site refactor; the full suite stays green.

## Scope — the five over-parametered services

| Service (before) | Params | Request struct (after) |
|---|---|---|
| `create_annual_planting` | 10 | `AnnualPlantingRequest` |
| `create_annual_planting_from_sowing` | 9 | *removed* → `AnnualPlantingRequest::from_sowing(..)` |
| `create_perennial_planting` | 10 | `PerennialPlantingRequest` |
| `record_yearly_harvest` | 6 | `YearlyHarvestRequest` |
| `record_treatment` | 8 | `TreatmentRequest` |

## Tasks / Subtasks

- [x] Task 1: Define the four request structs in `services.rs` (AC: 1, 2)
  - [x] `AnnualPlantingRequest` — required fields via `from_sowing(variety, location, strata, sown_on, area_m2, plants_count)` (method = `RaisedTransplant`) + `with_method`, `with_name`, `with_notes` builders. This absorbs the back-compat wrapper.
  - [x] `PerennialPlantingRequest` — `new(variety, location, strata, established_on, area_m2, plants_count)` + `with_expected_removal`, `with_name`, `with_notes`.
  - [x] `YearlyHarvestRequest` — `new(planting_id, year)` + `with_expected_yield`, `with_actual_yield`, `with_notes`.
  - [x] `TreatmentRequest` — `new(planting_id, applied_on, substance, product, dose, unit)` + `with_notes`.
- [x] Task 2: Rewrite the service bodies to take `(repo, request)` and drop the `#[allow(clippy::too_many_arguments)]` (AC: 2, 3)
  - [x] Remove `create_annual_planting_from_sowing`; callers use `AnnualPlantingRequest::from_sowing(..)`.
- [x] Task 3: Migrate every call site (AC: 1)
  - [x] `pomone-ui` wiring: `plantings.rs` (annual + perennial), `planting_detail.rs` (harvest + treatment).
  - [x] `pomone-app` non-test: `demo.rs`, `migration.rs` (tests) bodies + `services.rs` own test module.
  - [x] All `#[cfg(test)]` call sites across the view modules + `task_autogen.rs`.
- [x] Task 4: Verify (AC: 1–4)
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (389 passed, 0 failed — unchanged count).
  - [x] `grep` confirms no `too_many_arguments` left on the five services and no `create_annual_planting_from_sowing` code references remain (only a historical mention in a doc-comment).
  - [x] XDG-isolated `seed-demo` smoke run: 11 plantings created (10 annual + 1 perennial via the new request-struct path in `demo.rs`).

## Dev Notes

- **Builder over plain struct literal**: `new`/`from_sowing` seed the required fields; optionals (`name`, `notes`, `expected_removal_on`, yields) are `.with_*()` setters. This is what lets E1/E2 add an *optional* field without touching any call site — the stated goal of the story.
- **`method` stays a request field, not a service param.** The AC allowance for a third "injected date/clock" param is unused here — the agronomic date is a domain input and lives in the request.
- The service *bodies* are unchanged logic — only the parameter source changes (from positional args to `request.field`). No date maths, autogen, or validation is touched.

### Smoke procedure (XDG-isolated)

```sh
XDG_DATA_HOME=/tmp/pom05 XDG_CONFIG_HOME=/tmp/pom05 cargo run -p pomone-cli -- seed-demo
XDG_DATA_HOME=/tmp/pom05 XDG_CONFIG_HOME=/tmp/pom05 cargo run -p pomone-ui
```

## Completion Notes

- **Four request structs, builder-style.** Required fields go through `from_sowing`/`new`; optionals (`name`, `notes`, `expected_removal_on`, yields, `method`) are `#[must_use]` `with_*()` setters returning `Self`. This is what delivers the story's goal: E1/E2 can add an *optional* field as a new setter without touching a single call site.
- **The five services now take `(repo, request)`** — two params each — and every `#[allow(clippy::too_many_arguments)]` is gone; clippy is clean without it. `create_annual_planting_from_sowing` was deleted; its raise-then-transplant default lives on as `AnnualPlantingRequest::from_sowing(..)`.
- **Service bodies are byte-for-byte the same logic** — the leading `let Request { .. } = request;` destructure restores the exact locals the old positional bodies used, so date maths, autogen and validation are untouched. Test count is unchanged (389), confirming no behavior change.
- **Call-site migration (~46 sites, 15 files)** was fanned out to five parallel subagents over disjoint files, each given the exact new API + transformation rules; `services.rs`'s own test module and the two `pomone-ui` wiring sites were migrated by hand. UI wiring keeps the `Option` plumbing (`if let Some(x) = .. { req = req.with_x(x) }`) since the form fields are already optional.
- Verified: `fmt --check` clean, `clippy -D warnings` clean, `cargo test --workspace` 389/0, `seed-demo` smoke green.
