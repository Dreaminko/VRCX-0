use crate::state::AppState;

pub(super) fn db_config_bool(state: &AppState, key: &str) -> Option<bool> {
    state.runtime_context.config().get_bool(key, false).ok()
}

pub(super) fn app_language(state: &AppState) -> String {
    state
        .runtime_context
        .config()
        .get_string("appLanguage", "en")
        .unwrap_or_else(|_| "en".into())
        .to_ascii_lowercase()
}
