//! Needs list «Besoins» wiring (Epic 2, story 2.7). A read-only page: the only
//! callback is navigation, which loads the per-variety aggregation. Printing is
//! Epic 4, so there is no export callback here — the button is disabled in the
//! markup with a self-explaining tooltip.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::generated::MainWindow;
use crate::{refresh_needs, UiState};

/// Register the needs-page callbacks. Standard wiring shape (see `wiring/mod.rs`).
pub(crate) fn wire_needs(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    let state = Rc::clone(state);
    let weak = window.as_weak();
    window.on_navigate_needs(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        if let Err(e) = refresh_needs(&window, &mut state.borrow_mut()) {
            tracing::error!(error = %e, "failed to refresh needs list");
        }
        window.set_current_page(SharedString::from("needs"));
    });
}
