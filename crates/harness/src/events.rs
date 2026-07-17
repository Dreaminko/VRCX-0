use serde::Serialize;
use specta::Type;
use vrcx_0_application_core::RuntimeEventBus;

use crate::entities::Entity;

pub const EVENT_DELTA: &str = "assistantDelta";
pub const EVENT_TOOL_CALL: &str = "assistantToolCall";
pub const EVENT_TOOL_RESULT: &str = "assistantToolResult";
pub const EVENT_TURN_ENTITIES: &str = "assistantTurnEntities";
pub const EVENT_DONE: &str = "assistantDone";
pub const EVENT_ERROR: &str = "assistantError";

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantDeltaEvent {
    pub session_id: String,
    pub turn_id: String,
    pub text: String,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantToolCallEvent {
    pub session_id: String,
    pub turn_id: String,
    pub tool_call_id: String,
    pub name: String,
    pub args: String,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantToolResultEvent {
    pub session_id: String,
    pub turn_id: String,
    pub tool_call_id: String,
    pub ok: bool,
    pub summary: String,
    pub entities: Vec<Entity>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantTurnEntitiesEvent {
    pub session_id: String,
    pub turn_id: String,
    pub entities: Vec<Entity>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantDoneEvent {
    pub session_id: String,
    pub turn_id: String,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantErrorEvent {
    pub session_id: String,
    pub turn_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone)]
pub struct AssistantEmitter {
    bus: RuntimeEventBus,
    session_id: String,
    turn_id: String,
}

impl AssistantEmitter {
    pub fn new(bus: RuntimeEventBus, session_id: String, turn_id: String) -> Self {
        Self {
            bus,
            session_id,
            turn_id,
        }
    }

    pub fn delta(&self, text: &str) {
        self.bus.emit(
            EVENT_DELTA,
            AssistantDeltaEvent {
                session_id: self.session_id.clone(),
                turn_id: self.turn_id.clone(),
                text: text.to_string(),
            },
        );
    }

    pub fn tool_call(&self, tool_call_id: &str, name: &str, args: &str) {
        self.bus.emit(
            EVENT_TOOL_CALL,
            AssistantToolCallEvent {
                session_id: self.session_id.clone(),
                turn_id: self.turn_id.clone(),
                tool_call_id: tool_call_id.to_string(),
                name: name.to_string(),
                args: args.to_string(),
            },
        );
    }

    pub fn tool_result(&self, tool_call_id: &str, ok: bool, summary: &str, entities: &[Entity]) {
        self.bus.emit(
            EVENT_TOOL_RESULT,
            AssistantToolResultEvent {
                session_id: self.session_id.clone(),
                turn_id: self.turn_id.clone(),
                tool_call_id: tool_call_id.to_string(),
                ok,
                summary: summary.to_string(),
                entities: entities.to_vec(),
            },
        );
    }

    pub fn turn_entities(&self, entities: &[Entity]) {
        self.bus.emit(
            EVENT_TURN_ENTITIES,
            AssistantTurnEntitiesEvent {
                session_id: self.session_id.clone(),
                turn_id: self.turn_id.clone(),
                entities: entities.to_vec(),
            },
        );
    }

    pub fn done(&self) {
        self.bus.emit(
            EVENT_DONE,
            AssistantDoneEvent {
                session_id: self.session_id.clone(),
                turn_id: self.turn_id.clone(),
            },
        );
    }

    pub fn error(&self, code: &str, message: &str) {
        self.bus.emit(
            EVENT_ERROR,
            AssistantErrorEvent {
                session_id: self.session_id.clone(),
                turn_id: self.turn_id.clone(),
                code: code.to_string(),
                message: message.to_string(),
            },
        );
    }
}
