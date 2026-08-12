#![allow(non_snake_case)]

use vrcx_0_host_desktop::vrchat_log::{
    self, VrchatLogEntriesReadInput, VrchatLogEntriesReadOutput, VrchatLogFileOutput,
    VrchatLogTailReadInput,
};

use crate::error::AppError;

#[tauri::command]
#[specta::specta]
pub fn app__vrchat_log_files_list() -> Result<Vec<VrchatLogFileOutput>, AppError> {
    Ok(vrchat_log::files_list()?)
}

#[tauri::command]
#[specta::specta]
pub fn app__vrchat_log_entries_read(
    input: VrchatLogEntriesReadInput,
) -> Result<VrchatLogEntriesReadOutput, AppError> {
    Ok(vrchat_log::entries_read(input)?)
}

#[tauri::command]
#[specta::specta]
pub fn app__vrchat_log_tail_read(
    input: VrchatLogTailReadInput,
) -> Result<VrchatLogEntriesReadOutput, AppError> {
    Ok(vrchat_log::tail_read(input)?)
}
