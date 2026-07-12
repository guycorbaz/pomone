//! Strata screen wiring — extracted from `main.rs` (story 0.2). Shared helpers
//! (`UiState`, refreshes, delete executors, error rendering) stay in the
//! crate root and are reached through `crate::…`.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::generated::MainWindow;
use crate::{
    optional_text, parse_i32, refresh_bed_usage, refresh_strata, render_form_error,
    validate_optional_decimal, validate_required_name, FormError, PendingDelete, UiState,
};
use pomone_app::{create_strata, StrataInput};

/// Register every strata-screen callback on the window. Called once from
/// `main()`; standard wiring shape — see `wiring/mod.rs`.
pub(crate) fn wire_strata(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Strata navigation + create + delete ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_strata(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_strata(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh strata");
            }
            window.set_current_page(SharedString::from("strata"));
            window.set_strata_status_text(SharedString::from(""));
            window.set_strata_status_is_error(false);
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_create_strata(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_create_strata(&window, &mut s) {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_strata_status_text(SharedString::from(
                        i18n.t("status-strata-created"),
                    ));
                    window.set_strata_status_is_error(false);
                    window.set_new_strata_name(SharedString::from(""));
                    window.set_new_strata_description(SharedString::from(""));
                    window.set_new_strata_min_height(SharedString::from(""));
                    window.set_new_strata_max_height(SharedString::from(""));
                    if let Err(e) = refresh_strata(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh strata after create");
                    }
                    // Counts on the home page include strata; refresh too.
                    refresh_bed_usage(&window, &s.app, &s.runtime);
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), e);
                    window.set_strata_status_text(text);
                    window.set_strata_status_is_error(is_err);
                }
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_delete_strata(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.pending_delete = Some(PendingDelete::Strata(id.to_string()));
            window.set_confirm_message(SharedString::from(s.app.i18n().t("confirm-delete-strata")));
            window.set_confirm_visible(true);
        });
    }
}

fn try_create_strata(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let name = validate_required_name(&window.get_new_strata_name(), i18n)?;
    let description = optional_text(&window.get_new_strata_description());
    let min_height = validate_optional_decimal(&window.get_new_strata_min_height(), i18n)?;
    let max_height = validate_optional_decimal(&window.get_new_strata_max_height(), i18n)?;
    let sort_order =
        parse_i32(&window.get_new_strata_sort_order(), "sort order").map_err(FormError::Service)?;

    // Surface a friendly range message client-side; the domain would also
    // reject this but its error string is technical.
    if let (Some(min), Some(max)) = (min_height, max_height) {
        if min > max {
            return Err(FormError::Validation(i18n.t("error-height-range")));
        }
    }

    state
        .runtime
        .block_on(async {
            create_strata(
                state.app.repo(),
                StrataInput {
                    name,
                    description,
                    min_height_m: min_height,
                    max_height_m: max_height,
                    sort_order,
                },
            )
            .await
            .map(|_| ())
        })
        .map_err(FormError::Service)
}
