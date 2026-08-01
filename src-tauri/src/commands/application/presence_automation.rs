#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application_game::{
    presence_automation_rules_get, presence_automation_rules_set, PresenceAutomationRuleKind,
};
use vrcx_0_core::json::RawJson;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app__presence_automation_rules_get(
    state: State<'_, AppState>,
    kind: PresenceAutomationRuleKind,
) -> Result<Vec<RawJson>, AppError> {
    Ok(presence_automation_rules_get(
        state.runtime_context.config(),
        kind,
    )?)
}

#[tauri::command]
#[specta::specta]
pub fn app__presence_automation_rules_set(
    state: State<'_, AppState>,
    kind: PresenceAutomationRuleKind,
    rules: Vec<RawJson>,
) -> Result<Vec<RawJson>, AppError> {
    Ok(presence_automation_rules_set(
        state.runtime_context.config(),
        kind,
        rules,
    )?)
}
