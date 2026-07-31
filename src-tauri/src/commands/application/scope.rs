use vrcx_0_application_core::RuntimeAuthScopeSnapshot;

use crate::{error::AppError, state::AppState};

pub(crate) fn require_active_scope(
    state: &AppState,
    requirement: &str,
) -> Result<RuntimeAuthScopeSnapshot, AppError> {
    let scope = state.runtime_context.auth_scope.snapshot();
    if scope.active && !scope.current_user_id.trim().is_empty() {
        Ok(scope)
    } else {
        Err(vrcx_0_application_core::Error::Custom(format!(
            "{requirement} requires an authenticated session."
        ))
        .into())
    }
}
