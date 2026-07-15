//! Per-screen Slint callback wiring (story 0.1, architecture Slice 0).
//!
//! `main.rs` stays a thin bootstrap — build `App` + `UiState`, create the
//! `MainWindow`, run the initial refreshes — and delegates every screen's
//! callback registration to a module here. The pattern:
//!
//! - **one module per screen family** (`settings.rs`, later `plantings.rs`,
//!   `cultures.rs`, …), each exposing exactly one entry point:
//!   `pub(crate) fn wire_<screen>(window: &MainWindow, state: &Rc<RefCell<UiState>>)`
//!   that registers every `on_*` callback of that screen and nothing else;
//! - inside `wire_<screen>`, each callback block clones the `Rc` and a
//!   `window.as_weak()`, upgrading the weak at call time (see any block in
//!   `settings.rs` for the canonical shape);
//! - screen-local helpers move into the screen module; helpers shared across
//!   screens stay in the crate root and are reached through `crate::…`
//!   (private root items are visible to descendant modules).
//!
//! Adding a screen = a new module here + one `wire_*` call in `main()`.
//! Registering a callback directly in `main.rs` is an architecture
//! anti-pattern (see `_bmad-output/planning-artifacts/architecture.md`,
//! «Structure Patterns»).

pub(crate) mod agenda;
pub(crate) mod confirm;
pub(crate) mod crop_map;
pub(crate) mod cultures;
pub(crate) mod families;
pub(crate) mod home;
pub(crate) mod itk;
pub(crate) mod locations;
pub(crate) mod plan;
pub(crate) mod planting_detail;
pub(crate) mod plantings;
pub(crate) mod settings;
pub(crate) mod strata;
pub(crate) mod task_calendar;
pub(crate) mod task_form;
pub(crate) mod task_types;
