#![allow(non_snake_case)]

use tauri::State;

use crate::deep_link::DeepLinkAction;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app__drain_pending_deep_links(state: State<'_, AppState>) -> Vec<DeepLinkAction> {
    state.pending_deep_links.drain()
}
