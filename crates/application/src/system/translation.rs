use serde::{Deserialize, Serialize};
use vrcx_0_application_core::WebClient;
use vrcx_0_integrations::external_api::ExternalApiScope;
use vrcx_0_integrations::translation as protocol;
use vrcx_0_persistence::{config as config_store, DatabaseService};

use crate::{Error, Result};

pub use vrcx_0_integrations::translation::TranslationProvider;

const KEY_ENABLED: &str = "translationAPI";
const KEY_BIO_LANGUAGE: &str = "bioLanguage";
const KEY_API_TYPE: &str = "translationAPIType";
const KEY_API_KEY: &str = "translationAPIKey";
const KEY_ENDPOINT_ID: &str = "translationEndpointId";
const KEY_MODEL: &str = "translationAPIModel";
const KEY_PROMPT: &str = "translationAPIPrompt";
const KEY_REASONING_EFFORT: &str = "translationAPIReasoningEffort";

pub const DEFAULT_TRANSLATION_MODEL: &str = "gpt-4o-mini";

#[derive(Clone, Debug, Default, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TranslationOverrides {
    pub enabled: Option<bool>,
    pub api_type: Option<TranslationProvider>,
    pub key: Option<String>,
    pub endpoint_id: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub reasoning_effort: Option<String>,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TranslationTranslateInput {
    pub text: String,
    pub target_language: Option<String>,
    pub overrides: Option<TranslationOverrides>,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub text: String,
    pub detected_source_language: Option<String>,
    pub provider: TranslationProvider,
}

#[derive(Clone, Debug)]
pub struct OpenAiTranslationRequest {
    pub endpoint_id: String,
    pub model: String,
    pub prompt: Option<String>,
    pub reasoning_effort: Option<String>,
    pub target_language: String,
    pub text: String,
}

pub enum TranslationDispatch {
    Completed(TranslationResult),
    OpenAi(OpenAiTranslationRequest),
}

pub fn resolved_openai_translation_endpoint_id(db: &DatabaseService) -> Result<String> {
    Ok(config_store::get_string(db, KEY_ENDPOINT_ID, "")?
        .trim()
        .to_string())
}

fn override_or_config(
    db: &DatabaseService,
    value: Option<&String>,
    key: &str,
    default: &str,
) -> Result<String> {
    match value {
        Some(value) => Ok(value.clone()),
        None => Ok(config_store::get_string(db, key, default)?),
    }
}

pub async fn translate_text(
    db: &DatabaseService,
    web: &WebClient,
    input: TranslationTranslateInput,
) -> Result<TranslationDispatch> {
    let overrides = input.overrides.unwrap_or_default();
    let enabled = match overrides.enabled {
        Some(enabled) => enabled,
        None => config_store::get_bool(db, KEY_ENABLED, false)?,
    };
    if !enabled {
        return Err(Error::Custom("Translation API disabled.".into()));
    }

    let bio_language = config_store::get_string(db, KEY_BIO_LANGUAGE, "en")?;
    let target_language = input
        .target_language
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(bio_language);
    let target_language = if target_language.trim().is_empty() {
        "en".to_string()
    } else {
        target_language
    };
    let provider = match overrides.api_type {
        Some(provider) => provider,
        None => protocol::parse_translation_provider(&config_store::get_string(
            db,
            KEY_API_TYPE,
            "google",
        )?),
    };

    match provider {
        protocol::TranslationProvider::OpenAi => {
            let endpoint_id = match overrides.endpoint_id {
                Some(endpoint_id) => endpoint_id.trim().to_string(),
                None => resolved_openai_translation_endpoint_id(db)?,
            };
            let model = {
                let model = override_or_config(
                    db,
                    overrides.model.as_ref(),
                    KEY_MODEL,
                    DEFAULT_TRANSLATION_MODEL,
                )?;
                let model = model.trim().to_string();
                if model.is_empty() {
                    DEFAULT_TRANSLATION_MODEL.to_string()
                } else {
                    model
                }
            };
            let prompt = override_or_config(db, overrides.prompt.as_ref(), KEY_PROMPT, "")?;
            let reasoning_effort = override_or_config(
                db,
                overrides.reasoning_effort.as_ref(),
                KEY_REASONING_EFFORT,
                "",
            )?;
            Ok(TranslationDispatch::OpenAi(OpenAiTranslationRequest {
                endpoint_id,
                model,
                prompt: Some(prompt).filter(|value| !value.is_empty()),
                reasoning_effort: Some(reasoning_effort).filter(|value| !value.is_empty()),
                target_language,
                text: input.text,
            }))
        }
        protocol::TranslationProvider::Google | protocol::TranslationProvider::DeepL => {
            let key = override_or_config(db, overrides.key.as_ref(), KEY_API_KEY, "")?;
            if key.is_empty() {
                return Err(Error::Custom("No Translation API key configured.".into()));
            }

            let request = match provider {
                protocol::TranslationProvider::Google => {
                    protocol::google_translate_request(&key, &input.text, &target_language)
                        .map_err(|error| Error::Custom(error.to_string()))?
                }
                protocol::TranslationProvider::DeepL => {
                    protocol::deepl_translate_request(&key, &input.text, &target_language)
                        .map_err(|error| Error::Custom(error.to_string()))?
                }
                protocol::TranslationProvider::OpenAi => {
                    unreachable!("OpenAI translation is dispatched before HTTP translation")
                }
            };
            let response = web
                .execute_external_api(request, ExternalApiScope::Translation)
                .await?;
            if response.status != 200 {
                return Err(Error::Custom(format!(
                    "Translation API error: {}",
                    response.status
                )));
            }

            let outcome = match provider {
                protocol::TranslationProvider::Google => {
                    protocol::parse_google_translation_response(&response.data)
                }
                _ => protocol::parse_deepl_translation_response(&response.data),
            }
            .map_err(|error| Error::Custom(error.to_string()))?;

            Ok(TranslationDispatch::Completed(TranslationResult {
                text: outcome.text,
                detected_source_language: outcome.detected_source_language,
                provider,
            }))
        }
    }
}
