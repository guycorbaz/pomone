//! Locations screen wiring — extracted from `main.rs` (story 0.2). Shared helpers
//! (`UiState`, refreshes, delete executors, error rendering) stay in the
//! crate root and are reached through `crate::…`.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use slint::{ComponentHandle, SharedString};

use crate::generated::MainWindow;
use crate::{
    i32_to_usize, optional_text, refresh_locations, render_form_error, validate_positive_decimal,
    validate_required_name, FormError, PendingDelete, UiState,
};
use pomone_app::{
    create_location, get_location_for_edit, update_location, AppError, LocationEditForm,
    LocationInput,
};

/// Register every locations-screen callback on the window. Called once from
/// `main()`; standard wiring shape — see `wiring/mod.rs`.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_locations(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Locations navigation ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_locations(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_locations(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh locations");
            }
            window.set_current_page(SharedString::from("locations"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }

    // --- Create location ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_create_location(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let was_edit = window.get_loc_is_edit_mode();
            match try_save_location(&window, &mut s) {
                Ok(()) => {
                    let key = if was_edit {
                        "status-location-updated"
                    } else {
                        "status-location-created"
                    };
                    window.set_status_text(SharedString::from(s.app.i18n().t(key)));
                    window.set_status_is_error(false);
                    reset_location_form_to_create(&window, &mut s);
                    if let Err(e) = refresh_locations(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh locations after save");
                    }
                }
                // Reparenting under a descendant — show the localized message.
                Err(FormError::Service(AppError::Inconsistent(ref msg)))
                    if msg == "location_cycle" =>
                {
                    window.set_status_text(SharedString::from(
                        s.app.i18n().t("error-location-cycle"),
                    ));
                    window.set_status_is_error(true);
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), e);
                    window.set_status_text(text);
                    window.set_status_is_error(is_err);
                }
            }
        });
    }

    // --- Edit / cancel-edit a location (Lieux screen) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_edit_location(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = open_location_form_for_edit(&window, &mut s, &id) {
                tracing::error!(error = %e, "failed to open location edit form");
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_cancel_location_edit(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            reset_location_form_to_create(&window, &mut state.borrow_mut());
        });
    }

    // --- Delete a location (Lieux screen) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_delete_location(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.pending_delete = Some(PendingDelete::Location(id.to_string()));
            window.set_confirm_message(SharedString::from(
                s.app.i18n().t("confirm-delete-location"),
            ));
            window.set_confirm_visible(true);
        });
    }
}

/// Build the `LocationInput` from the form, then create or update depending on
/// the edit mode (`state.editing_location_id`).
fn try_save_location(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let kind_idx = i32_to_usize(window.get_loc_kind_index());
    let parent_idx = i32_to_usize(window.get_loc_parent_index());
    let kind_id_str = state
        .location_kind_ids
        .get(kind_idx)
        .ok_or_else(|| {
            FormError::Service(AppError::Inconsistent("no location kind selected".into()))
        })?
        .clone();
    let parent_id_str = state
        .parent_location_ids
        .get(parent_idx)
        .cloned()
        .unwrap_or_default();
    let name = validate_required_name(&window.get_new_loc_name(), i18n)?;
    let length_m = validate_positive_decimal(&window.get_new_loc_length(), i18n)?;
    let width_m = validate_positive_decimal(&window.get_new_loc_width(), i18n)?;
    let notes = optional_text(&window.get_new_loc_notes());

    let input = LocationInput {
        kind_id_str,
        name,
        length_m,
        width_m,
        parent_id_str,
        notes,
    };
    let editing_id = state.editing_location_id.clone();
    state
        .runtime
        .block_on(async {
            if editing_id.is_empty() {
                create_location(state.app.repo(), input).await.map(|_| ())
            } else {
                update_location(state.app.repo(), &editing_id, input).await
            }
        })
        .map_err(FormError::Service)
}

/// Clear the location form and drop back to create mode.
fn reset_location_form_to_create(window: &MainWindow, state: &mut UiState) {
    state.editing_location_id.clear();
    window.set_loc_is_edit_mode(false);
    window.set_new_loc_name(SharedString::from(""));
    window.set_new_loc_length(SharedString::from("5"));
    window.set_new_loc_width(SharedString::from("2"));
    window.set_new_loc_notes(SharedString::from(""));
    window.set_loc_kind_index(0);
    window.set_loc_parent_index(0);
}

/// Load one location into the form and switch it to edit mode.
fn open_location_form_for_edit(window: &MainWindow, state: &mut UiState, id: &str) -> Result<()> {
    let form: LocationEditForm = state
        .runtime
        .block_on(async { get_location_for_edit(state.app.repo(), id).await })
        .context("failed to load location for edit")?;

    let kind_idx = state
        .location_kind_ids
        .iter()
        .position(|k| k == &form.kind_id_str)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0));
    // parent_id_str is "" for a root; the parent dropdown's slot 0 is "(none)".
    let parent_idx = state
        .parent_location_ids
        .iter()
        .position(|p| p == &form.parent_id_str)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0));

    state.editing_location_id.clone_from(&form.id);
    window.set_loc_is_edit_mode(true);
    window.set_loc_kind_index(kind_idx);
    window.set_loc_parent_index(parent_idx);
    window.set_new_loc_name(SharedString::from(form.name));
    window.set_new_loc_length(SharedString::from(form.length));
    window.set_new_loc_width(SharedString::from(form.width));
    window.set_new_loc_notes(SharedString::from(form.notes));
    window.set_status_text(SharedString::from(""));
    window.set_status_is_error(false);
    Ok(())
}
