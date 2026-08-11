mod agent;
mod config;
mod endpoints;
mod entities;
mod error;
mod events;
mod playbook;
mod runtime;
mod session;
#[cfg(test)]
mod test_support;

pub use config::PlaybookMode;
pub use endpoints::{
    resolve_reasoning_effort, AssistantRuntimeSelection, AssistantRuntimeStatus, EndpointStore,
    LlmEndpointDetectModelsInput, LlmEndpointDto, LlmEndpointUpsertInput, LlmTranslateInput,
};
pub use entities::Entity;
pub use error::AssistantError;
pub use events::{
    AssistantDeltaEvent, AssistantDoneEvent, AssistantErrorEvent, AssistantToolCallEvent,
    AssistantToolResultEvent, AssistantTurnEntitiesEvent,
};
pub use runtime::{AssistantController, SendResult};
pub use session::{ActiveTurn, Message, Role, Session, SessionSummary, TurnStatus};
pub use vrcx_0_integrations::llm::{LlmEndpointDetectModelsResult, LlmModelReasoning};
