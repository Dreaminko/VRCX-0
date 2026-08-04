#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    resolved_openai_translation_endpoint_id, translate_text, OpenAiTranslationRequest,
    TranslationDispatch, TranslationProvider, TranslationResult, TranslationTranslateInput,
};
use vrcx_0_assistant::LlmTranslateInput;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn app__translation_translate(
    state: State<'_, AppState>,
    input: TranslationTranslateInput,
) -> Result<TranslationResult, AppError> {
    match translate_text(&state.db, &state.web, input).await? {
        TranslationDispatch::Completed(result) => Ok(result),
        TranslationDispatch::OpenAi(request) => translate_via_openai(&state, request).await,
    }
}

async fn translate_via_openai(
    state: &State<'_, AppState>,
    request: OpenAiTranslationRequest,
) -> Result<TranslationResult, AppError> {
    let OpenAiTranslationRequest {
        mut endpoint_id,
        model,
        prompt,
        reasoning_effort,
        target_language,
        text,
    } = request;

    let assistant = state.assistant().await?;
    if endpoint_id.is_empty() {
        assistant.endpoint_list().map_err(AppError::from)?;
        endpoint_id = resolved_openai_translation_endpoint_id(&state.db)?;
    }
    if endpoint_id.is_empty() || model.is_empty() {
        return Err(AppError::Custom(
            "Translation endpoint/model missing.".into(),
        ));
    }

    let translated = assistant
        .translate(LlmTranslateInput {
            endpoint_id,
            model,
            prompt,
            target_lang: target_language,
            text,
            reasoning_effort,
        })
        .await
        .map_err(AppError::from)?;

    Ok(TranslationResult {
        text: translated,
        detected_source_language: None,
        provider: TranslationProvider::OpenAi,
    })
}
