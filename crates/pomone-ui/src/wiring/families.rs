//! Families screen wiring — extracted from `main.rs` (story 0.2). Shared helpers
//! (`UiState`, refreshes, delete executors, error rendering) stay in the
//! crate root and are reached through `crate::…`.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::generated::MainWindow;
use crate::{
    color_chooser_palette, parse_hex_color, refresh_families, render_family_form_error,
    reset_families_form_to_create, FormError, PendingDelete, UiState,
};
use pomone_app::{create_family, get_family_for_edit, update_family, FamilyEditForm};

/// Register every families-screen callback on the window. Called once from
/// `main()`; standard wiring shape — see `wiring/mod.rs`.
pub(crate) fn wire_families(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Families: enter the page from the sidebar ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_families(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = open_families_page(&window, &mut s) {
                tracing::error!(error = %e, "failed to open families page");
            }
        });
    }
    // --- Families: Save (create OR update based on is_edit_mode) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_families_save(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_save_family_form(&window, &mut s) {
                Ok(()) => {
                    if let Err(e) = refresh_families(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh families after save");
                        return;
                    }
                    reset_families_form_to_create(&window, &mut s);
                }
                Err(e) => {
                    let (text, is_err) = render_family_form_error(s.app.i18n(), e);
                    window.set_families_status_text(text);
                    window.set_families_status_is_error(is_err);
                }
            }
        });
    }
    // --- Families: Cancel edit (return form to create mode) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_families_cancel_edit(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            reset_families_form_to_create(&window, &mut s);
        });
    }
    // --- Families: Edit a row → pre-fill the form in edit mode ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_families_edit_row(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = open_family_form_for_edit(&window, &mut s, &id) {
                tracing::error!(error = %e, "failed to open family edit form");
            }
        });
    }
    // --- Families: Delete a row (blocked at DB layer if in use) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_families_delete_row(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.pending_delete = Some(PendingDelete::Family(id.to_string()));
            window.set_confirm_message(SharedString::from(s.app.i18n().t("confirm-delete-family")));
            window.set_confirm_visible(true);
        });
    }
}

/// First-time entry into the Families page: load the list, blank form.
fn open_families_page(window: &MainWindow, state: &mut UiState) -> Result<()> {
    window.set_families_color_palette(ModelRc::new(VecModel::from(color_chooser_palette())));
    refresh_families(window, state)?;
    reset_families_form_to_create(window, state);
    window.set_current_page(SharedString::from("families"));
    Ok(())
}

/// Load one family into the form and switch to edit mode.
fn open_family_form_for_edit(window: &MainWindow, state: &mut UiState, id: &str) -> Result<()> {
    refresh_families(window, state)?;
    let form: FamilyEditForm = state
        .runtime
        .block_on(async { get_family_for_edit(state.app.repo(), id).await })
        .context("failed to load family for edit")?;
    state.editing_family_id.clone_from(&form.id);
    window.set_families_is_edit_mode(true);
    window.set_families_form_color_preview(parse_hex_color(&form.color));
    window.set_families_form_name(SharedString::from(form.name));
    window.set_families_form_latin(SharedString::from(form.latin_name));
    window.set_families_form_description(SharedString::from(form.description));
    window.set_families_form_color(SharedString::from(form.color));
    window.set_families_status_text(SharedString::from(""));
    window.set_families_status_is_error(false);
    Ok(())
}

/// Persist the Families form (create OR update based on edit mode).
fn try_save_family_form(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let name = window.get_families_form_name().to_string();
    if name.trim().is_empty() {
        return Err(FormError::Validation(i18n.t("error-name-required")));
    }
    let latin = window.get_families_form_latin().to_string();
    let description = window.get_families_form_description().to_string();
    let color = window.get_families_form_color().to_string();
    if color.trim().is_empty() {
        return Err(FormError::Validation(i18n.t("error-family-color-required")));
    }

    if window.get_families_is_edit_mode() {
        let id = state.editing_family_id.clone();
        if id.is_empty() {
            return Err(FormError::Validation(
                i18n.t("error-family-edit-id-missing"),
            ));
        }
        state
            .runtime
            .block_on(async {
                update_family(
                    state.app.repo(),
                    &id,
                    name.trim(),
                    latin.trim(),
                    description.trim(),
                    color.trim(),
                )
                .await
            })
            .map_err(FormError::Service)?;
    } else {
        state
            .runtime
            .block_on(async {
                create_family(
                    state.app.repo(),
                    name.trim(),
                    latin.trim(),
                    description.trim(),
                    color.trim(),
                )
                .await
                .map(|_| ())
            })
            .map_err(FormError::Service)?;
    }
    Ok(())
}
