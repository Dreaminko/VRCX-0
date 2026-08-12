use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, Weak};

use serde::Serialize;
use specta::Type;
use vrcx_0_persistence::assistant;
use vrcx_0_persistence::DatabaseService;

use crate::config::PlaybookMode;
use crate::endpoints::AssistantRuntimeSelection;
use crate::entities::Entity;

const SESSION_CONTENT_CACHE_CAPACITY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub seq: u64,
    pub role: Role,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum TurnStatus {
    Running,
    Done,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTurn {
    pub turn_id: String,
    pub status: TurnStatus,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub active_turn: Option<ActiveTurn>,
    pub endpoint_id: Option<String>,
    pub model: Option<String>,
    pub allow_writes: bool,
    pub playbook_mode: PlaybookMode,
    pub entity_panel_open: bool,
    pub surfaced_entities: Vec<Entity>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub busy: bool,
    pub updated_at: String,
}

#[derive(Clone)]
struct StoredSession {
    owner_user_id: String,
    title: String,
    active_turn: Option<ActiveTurn>,
    endpoint_id: Option<String>,
    model: Option<String>,
    allow_writes: bool,
    playbook_mode: PlaybookMode,
    entity_panel_open: bool,
    surfaced_entities: Vec<Entity>,
    created_at: String,
    updated_at: String,
}

impl StoredSession {
    fn materialize(&self, id: String, messages: Vec<Message>) -> Session {
        Session {
            id,
            title: self.title.clone(),
            messages,
            active_turn: self.active_turn.clone(),
            endpoint_id: self.endpoint_id.clone(),
            model: self.model.clone(),
            allow_writes: self.allow_writes,
            playbook_mode: self.playbook_mode,
            entity_panel_open: self.entity_panel_open,
            surfaced_entities: self.surfaced_entities.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

struct SessionContent {
    messages: Vec<Message>,
    pending_writes: usize,
    write_failed: bool,
}

impl SessionContent {
    fn loaded(messages: Vec<Message>) -> Self {
        Self {
            messages,
            pending_writes: 0,
            write_failed: false,
        }
    }
}

#[derive(Default)]
struct SessionStoreState {
    sessions: HashMap<String, StoredSession>,
    contents: HashMap<String, SessionContent>,
    content_lru: VecDeque<String>,
}

impl SessionStoreState {
    fn touch_content(&mut self, session_id: &str) {
        self.content_lru
            .retain(|cached_session_id| cached_session_id != session_id);
        self.content_lru.push_back(session_id.to_string());
    }

    fn remove_content(&mut self, session_id: &str) {
        self.contents.remove(session_id);
        self.content_lru
            .retain(|cached_session_id| cached_session_id != session_id);
    }

    fn evict_contents(&mut self, can_reload: bool) {
        if !can_reload {
            return;
        }
        while self.contents.len() > SESSION_CONTENT_CACHE_CAPACITY {
            let candidate = self.content_lru.iter().position(|session_id| {
                let running = self
                    .sessions
                    .get(session_id)
                    .and_then(|session| session.active_turn.as_ref())
                    .is_some_and(|turn| matches!(turn.status, TurnStatus::Running));
                self.contents.get(session_id).is_some_and(|content| {
                    !running && content.pending_writes == 0 && !content.write_failed
                })
            });
            let Some(candidate) = candidate else {
                break;
            };
            let Some(session_id) = self.content_lru.remove(candidate) else {
                break;
            };
            self.contents.remove(&session_id);
        }
    }

    fn materialize_loaded(&mut self, session_id: &str) -> Option<Session> {
        let session = self.sessions.get(session_id)?.clone();
        let messages = self.contents.get(session_id)?.messages.clone();
        self.touch_content(session_id);
        Some(session.materialize(session_id.to_string(), messages))
    }
}

#[derive(Default)]
pub struct SessionStore {
    state: Mutex<SessionStoreState>,
    content_loads: Mutex<HashMap<String, Weak<Mutex<()>>>>,
    loaded_owners: Mutex<HashSet<String>>,
    seq: Mutex<u64>,
    db: Option<Arc<DatabaseService>>,
}

impl SessionStore {
    pub fn with_db(db: Arc<DatabaseService>) -> Self {
        Self {
            state: Mutex::new(SessionStoreState::default()),
            content_loads: Mutex::new(HashMap::new()),
            loaded_owners: Mutex::new(HashSet::new()),
            seq: Mutex::new(0),
            db: Some(db),
        }
    }

    fn ensure_owner_loaded(&self, owner_user_id: &str) {
        let owner_user_id = owner_user_id.trim();
        let mut loaded_owners = self.loaded_owners.lock().unwrap();
        if loaded_owners.contains(owner_user_id) {
            return;
        }
        let Some(db) = self.db.as_ref() else {
            loaded_owners.insert(owner_user_id.to_string());
            return;
        };
        match assistant::assistant_sessions_load(db, owner_user_id) {
            Ok(persisted) => {
                let mut state = self.state.lock().unwrap();
                for entry in persisted {
                    if state.sessions.contains_key(&entry.id) {
                        continue;
                    }
                    state.sessions.insert(
                        entry.id.clone(),
                        StoredSession {
                            owner_user_id: entry.owner_user_id,
                            title: entry.title,
                            active_turn: None,
                            endpoint_id: optional_string(entry.endpoint_id),
                            model: optional_string(entry.model),
                            allow_writes: entry.allow_writes,
                            playbook_mode: PlaybookMode::parse(&entry.playbook_mode),
                            entity_panel_open: entry.entity_panel_open,
                            surfaced_entities: serde_json::from_str(&entry.surfaced_entities)
                                .unwrap_or_default(),
                            created_at: entry.created_at,
                            updated_at: entry.updated_at,
                        },
                    );
                }
                loaded_owners.insert(owner_user_id.to_string());
            }
            Err(error) => {
                tracing::warn!(%error, "assistant: failed to load persisted sessions");
            }
        }
    }

    fn content_load(&self, session_id: &str) -> Arc<Mutex<()>> {
        let mut loads = self.content_loads.lock().unwrap();
        if let Some(load) = loads.get(session_id).and_then(Weak::upgrade) {
            return load;
        }
        loads.retain(|_, load| load.strong_count() > 0);
        let load = Arc::new(Mutex::new(()));
        loads.insert(session_id.to_string(), Arc::downgrade(&load));
        load
    }

    fn persisted_messages(
        &self,
        owner_user_id: &str,
        session_id: &str,
    ) -> vrcx_0_persistence::Result<Vec<Message>> {
        let Some(db) = self.db.as_ref() else {
            return Ok(Vec::new());
        };
        let history = assistant::assistant_session_messages_load(db, owner_user_id, session_id)?
            .into_iter()
            .map(|message| Message {
                id: message.id,
                seq: message.seq.max(0) as u64,
                role: parse_role(&message.role),
                content: message.content,
                created_at: message.created_at,
            })
            .collect::<Vec<_>>();
        if let Some(max_seq) = history.iter().map(|message| message.seq).max() {
            let mut seq = self.seq.lock().unwrap();
            *seq = (*seq).max(max_seq);
        }
        Ok(history)
    }

    fn with_loaded_session<R>(
        &self,
        session_id: &str,
        operation: impl FnOnce(&mut SessionStoreState) -> Option<R>,
    ) -> vrcx_0_persistence::Result<Option<R>> {
        let mut operation = Some(operation);
        let owner_user_id = {
            let mut state = self.state.lock().unwrap();
            let Some(session) = state.sessions.get(session_id) else {
                return Ok(None);
            };
            let owner_user_id = session.owner_user_id.clone();
            if state.contents.contains_key(session_id) {
                let result = operation.take().expect("session operation is available")(&mut state);
                state.evict_contents(self.db.is_some());
                return Ok(result);
            }
            owner_user_id
        };
        let loaded = self.persisted_messages(&owner_user_id, session_id)?;
        let mut state = self.state.lock().unwrap();
        if !state.sessions.contains_key(session_id) {
            return Ok(None);
        }
        state
            .contents
            .entry(session_id.to_string())
            .or_insert_with(|| SessionContent::loaded(loaded));
        let result = operation.expect("session operation is available")(&mut state);
        state.evict_contents(self.db.is_some());
        Ok(result)
    }

    fn upsert_row(
        &self,
        owner_user_id: &str,
        id: &str,
        title: &str,
        created_at: &str,
        updated_at: &str,
    ) -> bool {
        let Some(db) = self.db.as_ref() else {
            return false;
        };
        match assistant::assistant_session_upsert(
            db,
            owner_user_id,
            id,
            title,
            created_at,
            updated_at,
        ) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(%error, "assistant: failed to persist session");
                false
            }
        }
    }

    fn persist_session(&self, id: &str, session: &StoredSession) {
        self.upsert_row(
            &session.owner_user_id,
            id,
            &session.title,
            &session.created_at,
            &session.updated_at,
        );
        persist_runtime(self.db.as_deref(), id, session);
    }

    fn persist_message(
        &self,
        id: &str,
        title: &str,
        created_at: &str,
        updated_at: &str,
        message: &Message,
        owner_user_id: &str,
    ) -> bool {
        let session_persisted = self.upsert_row(owner_user_id, id, title, created_at, updated_at);
        let Some(db) = self.db.as_ref() else {
            return false;
        };
        let message_persisted = match assistant::assistant_message_insert(
            db,
            &message.id,
            id,
            message.seq as i64,
            role_str(message.role),
            &message.content,
            &message.created_at,
        ) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(%error, "assistant: failed to persist message");
                false
            }
        };
        session_persisted && message_persisted
    }

    pub fn next_seq(&self) -> u64 {
        let mut guard = self.seq.lock().unwrap();
        *guard += 1;
        *guard
    }

    fn insert_new(
        &self,
        owner_user_id: &str,
        id: String,
        runtime: AssistantRuntimeSelection,
    ) -> Session {
        let load = self.content_load(&id);
        let _load = load.lock().unwrap();
        let now = now_rfc3339();
        let stored = StoredSession {
            owner_user_id: owner_user_id.trim().to_string(),
            title: String::new(),
            active_turn: None,
            endpoint_id: normalize_optional(runtime.endpoint_id),
            model: normalize_optional(runtime.model),
            allow_writes: runtime.allow_writes,
            playbook_mode: runtime.playbook_mode,
            entity_panel_open: false,
            surfaced_entities: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        {
            let mut state = self.state.lock().unwrap();
            state.sessions.insert(id.clone(), stored.clone());
            state
                .contents
                .insert(id.clone(), SessionContent::loaded(Vec::new()));
            state.touch_content(&id);
            state.evict_contents(self.db.is_some());
        }
        self.persist_session(&id, &stored);
        stored.materialize(id, Vec::new())
    }

    pub fn create_session_with_runtime(
        &self,
        owner_user_id: &str,
        runtime: AssistantRuntimeSelection,
    ) -> Session {
        self.ensure_owner_loaded(owner_user_id);
        self.insert_new(owner_user_id, format!("ses_{}", random_hex()), runtime)
    }

    pub fn ensure_session_with_runtime(
        &self,
        owner_user_id: &str,
        session_id: Option<String>,
        runtime: AssistantRuntimeSelection,
    ) -> vrcx_0_persistence::Result<Option<Session>> {
        self.ensure_owner_loaded(owner_user_id);
        let Some(id) = session_id else {
            return Ok(Some(self.insert_new(
                owner_user_id,
                format!("ses_{}", random_hex()),
                runtime,
            )));
        };
        let load = self.content_load(&id);
        let _load = load.lock().unwrap();
        let visible = {
            let state = self.state.lock().unwrap();
            state
                .sessions
                .get(&id)
                .map(|session| owner_visible(&session.owner_user_id, owner_user_id))
        };
        if visible == Some(false) {
            return Ok(None);
        }
        if visible.is_none() {
            drop(_load);
            return Ok(Some(self.insert_new(owner_user_id, id, runtime)));
        }
        let mut seeded = false;
        let session = self.with_loaded_session(&id, |state| {
            let session = state.sessions.get_mut(&id)?;
            if session.endpoint_id.is_none() && session.model.is_none() {
                apply_runtime(session, runtime);
                session.updated_at = now_rfc3339();
                seeded = true;
            }
            state.materialize_loaded(&id)
        })?;
        if seeded {
            let stored = self.state.lock().unwrap().sessions.get(&id).cloned();
            if let Some(stored) = stored {
                persist_runtime(self.db.as_deref(), &id, &stored);
            }
        }
        Ok(session)
    }

    pub fn get(
        &self,
        owner_user_id: &str,
        session_id: &str,
    ) -> vrcx_0_persistence::Result<Option<Session>> {
        if !self.is_visible_to(session_id, owner_user_id) {
            return Ok(None);
        }
        let load = self.content_load(session_id);
        let _load = load.lock().unwrap();
        self.with_loaded_session(session_id, |state| state.materialize_loaded(session_id))
    }

    pub(crate) fn get_unscoped(&self, session_id: &str) -> Option<Session> {
        self.state.lock().unwrap().materialize_loaded(session_id)
    }

    pub fn list(&self, owner_user_id: &str) -> Vec<SessionSummary> {
        self.ensure_owner_loaded(owner_user_id);
        let state = self.state.lock().unwrap();
        let mut summaries: Vec<SessionSummary> = state
            .sessions
            .iter()
            .filter(|(_, session)| owner_visible(&session.owner_user_id, owner_user_id))
            .map(|(id, session)| SessionSummary {
                id: id.clone(),
                title: session.title.clone(),
                busy: session
                    .active_turn
                    .as_ref()
                    .is_some_and(|turn| matches!(turn.status, TurnStatus::Running)),
                updated_at: session.updated_at.clone(),
            })
            .collect();
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        summaries
    }

    pub fn delete(&self, owner_user_id: &str, session_id: &str) {
        if !self.is_visible_to(session_id, owner_user_id) {
            return;
        }
        let load = self.content_load(session_id);
        let _load = load.lock().unwrap();
        {
            let mut state = self.state.lock().unwrap();
            state.sessions.remove(session_id);
            state.remove_content(session_id);
        }
        if let Some(db) = self.db.as_ref() {
            if let Err(error) = assistant::assistant_session_delete(db, owner_user_id, session_id) {
                tracing::warn!(%error, "assistant: failed to delete persisted session");
            }
        }
    }

    pub fn set_active_turn(&self, session_id: &str, turn: Option<ActiveTurn>) {
        let mut state = self.state.lock().unwrap();
        if let Some(session) = state.sessions.get_mut(session_id) {
            session.active_turn = turn;
            session.updated_at = now_rfc3339();
            if state.contents.contains_key(session_id) {
                state.touch_content(session_id);
            }
            state.evict_contents(self.db.is_some());
        }
    }

    /// Whether `turn_id` is still the session's active turn — false once a newer
    /// turn has taken over, so a superseded turn can bow out without clobbering it.
    pub fn is_current_turn(&self, session_id: &str, turn_id: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .sessions
            .get(session_id)
            .and_then(|session| session.active_turn.as_ref())
            .is_some_and(|turn| turn.turn_id == turn_id)
    }

    pub fn push_message(
        &self,
        session_id: &str,
        role: Role,
        content: String,
    ) -> vrcx_0_persistence::Result<bool> {
        let load = self.content_load(session_id);
        let _load = load.lock().unwrap();
        let row = self.with_loaded_session(session_id, |state| {
            let session = state.sessions.get_mut(session_id)?;
            let now = now_rfc3339();
            if matches!(role, Role::User) && session.title.is_empty() {
                session.title = derive_title(&content);
            }
            let message = Message {
                id: format!("msg_{}", random_hex()),
                seq: self.next_seq(),
                role,
                content,
                created_at: now.clone(),
            };
            session.updated_at = now;
            let owner_user_id = session.owner_user_id.clone();
            let title = session.title.clone();
            let created_at = session.created_at.clone();
            let updated_at = session.updated_at.clone();
            let cached = state.contents.get_mut(session_id)?;
            cached.messages.push(message.clone());
            cached.pending_writes = cached.pending_writes.saturating_add(1);
            state.touch_content(session_id);
            Some((owner_user_id, title, created_at, updated_at, message))
        })?;
        let Some((owner_user_id, title, created_at, updated_at, message)) = row else {
            return Ok(false);
        };
        let persisted = self.persist_message(
            session_id,
            &title,
            &created_at,
            &updated_at,
            &message,
            &owner_user_id,
        );
        let mut state = self.state.lock().unwrap();
        if let Some(cached) = state.contents.get_mut(session_id) {
            cached.pending_writes = cached.pending_writes.saturating_sub(1);
            if !persisted {
                cached.write_failed = true;
            }
        }
        state.evict_contents(self.db.is_some());
        Ok(true)
    }

    pub fn history(&self, session_id: &str) -> Vec<Message> {
        let mut state = self.state.lock().unwrap();
        let history = state
            .contents
            .get(session_id)
            .map(|content| content.messages.clone())
            .unwrap_or_default();
        if state.contents.contains_key(session_id) {
            state.touch_content(session_id);
        }
        history
    }

    /// Persist the entities a turn surfaced (and auto-open the panel for them),
    /// so a reopened session restores its right-panel contents.
    pub fn set_surfaced_entities(&self, session_id: &str, entities: &[Entity]) {
        // Mutate and persist under one lock so the in-memory change and the DB
        // write stay ordered together — a manual toggle racing a turn end can't
        // land its UPDATE out of order and desync the persisted panel state.
        let mut state = self.state.lock().unwrap();
        let Some(session) = state.sessions.get_mut(session_id) else {
            return;
        };
        session.surfaced_entities = entities.to_vec();
        if !entities.is_empty() {
            session.entity_panel_open = true;
        }
        persist_ui_state(
            self.db.as_deref(),
            session_id,
            session.entity_panel_open,
            &session.surfaced_entities,
        );
    }

    pub fn set_entity_panel_open(&self, session_id: &str, open: bool) {
        let mut state = self.state.lock().unwrap();
        let Some(session) = state.sessions.get_mut(session_id) else {
            return;
        };
        session.entity_panel_open = open;
        persist_ui_state(
            self.db.as_deref(),
            session_id,
            open,
            &session.surfaced_entities,
        );
    }

    pub fn set_runtime(
        &self,
        owner_user_id: &str,
        session_id: &str,
        runtime: AssistantRuntimeSelection,
    ) -> vrcx_0_persistence::Result<Option<Session>> {
        if !self.is_visible_to(session_id, owner_user_id) {
            return Ok(None);
        }
        let load = self.content_load(session_id);
        let _load = load.lock().unwrap();
        let updated = self.with_loaded_session(session_id, |state| {
            let session = state.sessions.get_mut(session_id)?;
            apply_runtime(session, runtime);
            session.updated_at = now_rfc3339();
            let stored = session.clone();
            let materialized = state.materialize_loaded(session_id)?;
            Some((stored, materialized))
        })?;
        let Some((stored, materialized)) = updated else {
            return Ok(None);
        };
        persist_runtime(self.db.as_deref(), session_id, &stored);
        Ok(Some(materialized))
    }

    pub fn is_visible_to(&self, session_id: &str, owner_user_id: &str) -> bool {
        self.ensure_owner_loaded(owner_user_id);
        self.is_loaded_session_visible_to(session_id, owner_user_id)
    }

    fn is_loaded_session_visible_to(&self, session_id: &str, owner_user_id: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .sessions
            .get(session_id)
            .is_some_and(|session| owner_visible(&session.owner_user_id, owner_user_id))
    }
}

fn owner_visible(session_owner: &str, owner_user_id: &str) -> bool {
    session_owner.is_empty() || session_owner == owner_user_id.trim()
}

fn persist_ui_state(
    db: Option<&DatabaseService>,
    session_id: &str,
    open: bool,
    entities: &[Entity],
) {
    let Some(db) = db else {
        return;
    };
    let json = serde_json::to_string(entities).unwrap_or_else(|_| "[]".into());
    if let Err(error) = assistant::assistant_session_set_ui_state(db, session_id, open, &json) {
        tracing::warn!(%error, "assistant: failed to persist panel state");
    }
}

fn persist_runtime(db: Option<&DatabaseService>, session_id: &str, session: &StoredSession) {
    let Some(db) = db else {
        return;
    };
    if let Err(error) = assistant::assistant_session_set_runtime(
        db,
        session_id,
        session.endpoint_id.as_deref(),
        session.model.as_deref(),
        session.allow_writes,
        session.playbook_mode.as_config_str(),
    ) {
        tracing::warn!(%error, "assistant: failed to persist runtime selection");
    }
}

fn apply_runtime(session: &mut StoredSession, runtime: AssistantRuntimeSelection) {
    session.endpoint_id = normalize_optional(runtime.endpoint_id);
    session.model = normalize_optional(runtime.model);
    session.allow_writes = runtime.allow_writes;
    session.playbook_mode = runtime.playbook_mode;
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn optional_string(value: String) -> Option<String> {
    normalize_optional(Some(value))
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

fn parse_role(role: &str) -> Role {
    match role {
        "assistant" => Role::Assistant,
        _ => Role::User,
    }
}

fn derive_title(content: &str) -> String {
    let trimmed = content.trim();
    let title: String = trimmed.chars().take(40).collect();
    if trimmed.chars().count() > 40 {
        format!("{title}…")
    } else {
        title
    }
}

pub fn random_hex() -> String {
    let mut bytes = [0u8; 12];
    if getrandom::fill(&mut bytes).is_err() {
        return "000000000000".into();
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::unique_test_database_path;

    const TEST_OWNER: &str = "usr_test";

    fn test_db() -> Arc<DatabaseService> {
        Arc::new(DatabaseService::new(&unique_test_database_path("vrcx-0-assistant")).unwrap())
    }

    fn create_test_session(store: &SessionStore) -> Session {
        store.create_session_with_runtime(TEST_OWNER, AssistantRuntimeSelection::default())
    }

    #[test]
    fn reopened_session_keeps_history_for_followups() {
        let db = test_db();
        let session = {
            let store = SessionStore::with_db(db.clone());
            let session = create_test_session(&store);
            store
                .push_message(&session.id, Role::User, "who do I play with?".into())
                .unwrap();
            store
                .push_message(&session.id, Role::Assistant, "Alice and Bob.".into())
                .unwrap();
            session
        };

        // Simulate an app restart: a fresh store over the same database must
        // hydrate the prior turns so the next question is sent with context.
        let reopened = SessionStore::with_db(db);
        let history = reopened
            .get(TEST_OWNER, &session.id)
            .unwrap()
            .unwrap()
            .messages;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, Role::User);
        assert_eq!(history[0].content, "who do I play with?");
        assert_eq!(history[1].role, Role::Assistant);
        assert_eq!(history[1].content, "Alice and Bob.");
    }

    #[test]
    fn message_load_failure_retries_without_caching_partial_history() {
        let db = test_db();
        let session_id = {
            let store = SessionStore::with_db(db.clone());
            let session = create_test_session(&store);
            assert!(store
                .push_message(&session.id, Role::User, "persisted".into())
                .unwrap());
            session.id
        };
        let reopened = SessionStore::with_db(db.clone());
        assert_eq!(reopened.list(TEST_OWNER).len(), 1);

        let _frozen = db.freeze_for_migration().unwrap();
        assert!(reopened
            .push_message(&session_id, Role::User, "must not append".into())
            .is_err());
        assert!(!reopened
            .state
            .lock()
            .unwrap()
            .contents
            .contains_key(&session_id));

        db.reopen_after_migration_abort().unwrap();
        let restored = reopened.get(TEST_OWNER, &session_id).unwrap().unwrap();
        assert_eq!(restored.messages.len(), 1);
        assert_eq!(restored.messages[0].content, "persisted");
        assert!(reopened
            .push_message(&session_id, Role::User, "after recovery".into())
            .unwrap());
        assert_eq!(reopened.history(&session_id).len(), 2);
    }

    #[test]
    fn session_content_cache_evicts_clean_inactive_histories() {
        let store = SessionStore::with_db(test_db());
        let mut session_ids = Vec::new();
        for _ in 0..SESSION_CONTENT_CACHE_CAPACITY + 3 {
            session_ids.push(create_test_session(&store).id);
        }
        assert_eq!(
            store.state.lock().unwrap().contents.len(),
            SESSION_CONTENT_CACHE_CAPACITY
        );

        let oldest = &session_ids[0];
        assert!(store.get(TEST_OWNER, oldest).unwrap().is_some());
        let state = store.state.lock().unwrap();
        assert_eq!(state.contents.len(), SESSION_CONTENT_CACHE_CAPACITY);
        assert!(state.contents.contains_key(oldest));
    }

    #[test]
    fn failed_message_write_is_not_evicted() {
        let db = test_db();
        let store = SessionStore::with_db(db.clone());
        let session = create_test_session(&store);
        let _frozen = db.freeze_for_migration().unwrap();

        assert!(store
            .push_message(&session.id, Role::User, "memory only".into())
            .unwrap());
        for _ in 0..SESSION_CONTENT_CACHE_CAPACITY + 3 {
            create_test_session(&store);
        }
        let state = store.state.lock().unwrap();
        let content = state.contents.get(&session.id).unwrap();
        assert!(content.write_failed);
        assert_eq!(content.messages[0].content, "memory only");
        drop(state);

        db.reopen_after_migration_abort().unwrap();
    }

    #[test]
    fn session_snapshot_keeps_title_and_messages_consistent_during_push() {
        let store = Arc::new(SessionStore::with_db(test_db()));
        let session = create_test_session(&store);
        let session_id = session.id.clone();
        let writer_store = Arc::clone(&store);
        let writer_session_id = session_id.clone();
        let writer = std::thread::spawn(move || {
            writer_store
                .push_message(&writer_session_id, Role::User, "first message".into())
                .unwrap();
        });

        while !writer.is_finished() {
            let snapshot = store.get(TEST_OWNER, &session_id).unwrap().unwrap();
            if !snapshot.messages.is_empty() {
                assert_eq!(snapshot.title, "first message");
            }
        }
        writer.join().unwrap();
        let snapshot = store.get(TEST_OWNER, &session_id).unwrap().unwrap();
        assert_eq!(snapshot.title, "first message");
        assert_eq!(snapshot.messages.len(), 1);
    }

    #[test]
    fn reopened_session_restores_panel_state() {
        let db = test_db();
        let session_id = {
            let store = SessionStore::with_db(db.clone());
            let session = create_test_session(&store);
            store.set_surfaced_entities(
                &session.id,
                &[Entity {
                    kind: "user".into(),
                    id: "usr_1".into(),
                    display_name: "Alice".into(),
                }],
            );
            session.id
        };

        // Surfacing entities auto-opens the panel; both must survive a restart.
        let reopened = SessionStore::with_db(db)
            .get(TEST_OWNER, &session_id)
            .unwrap()
            .unwrap();
        assert!(reopened.entity_panel_open);
        assert_eq!(reopened.surfaced_entities.len(), 1);
        assert_eq!(reopened.surfaced_entities[0].id, "usr_1");
        assert_eq!(reopened.surfaced_entities[0].display_name, "Alice");
    }

    #[test]
    fn runtime_selection_round_trips_and_lazy_seeds_old_sessions() {
        let db = test_db();
        let session_id = {
            let store = SessionStore::with_db(db.clone());
            let session = create_test_session(&store);
            store
                .set_runtime(
                    TEST_OWNER,
                    &session.id,
                    AssistantRuntimeSelection {
                        endpoint_id: Some("ep_1".into()),
                        model: Some("model-a".into()),
                        allow_writes: true,
                        playbook_mode: PlaybookMode::Guided,
                    },
                )
                .unwrap()
                .unwrap()
                .id
        };

        let reopened = SessionStore::with_db(db.clone())
            .get(TEST_OWNER, &session_id)
            .unwrap()
            .unwrap();
        assert_eq!(reopened.endpoint_id.as_deref(), Some("ep_1"));
        assert_eq!(reopened.model.as_deref(), Some("model-a"));
        assert!(reopened.allow_writes);
        assert_eq!(reopened.playbook_mode, PlaybookMode::Guided);

        let old_session_id = {
            let store = SessionStore::with_db(db.clone());
            create_test_session(&store).id
        };
        let store = SessionStore::with_db(db);
        let seeded = store
            .ensure_session_with_runtime(
                TEST_OWNER,
                Some(old_session_id),
                AssistantRuntimeSelection {
                    endpoint_id: Some("ep_seed".into()),
                    model: Some("seed-model".into()),
                    allow_writes: false,
                    playbook_mode: PlaybookMode::Open,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(seeded.endpoint_id.as_deref(), Some("ep_seed"));
        assert_eq!(seeded.model.as_deref(), Some("seed-model"));
        assert_eq!(seeded.playbook_mode, PlaybookMode::Open);
    }

    #[test]
    fn empty_surfaced_entities_clear_prior_references() {
        let db = test_db();
        let session_id = {
            let store = SessionStore::with_db(db.clone());
            let session = create_test_session(&store);
            store.set_surfaced_entities(
                &session.id,
                &[Entity {
                    kind: "user".into(),
                    id: "usr_1".into(),
                    display_name: "Alice".into(),
                }],
            );
            store.set_surfaced_entities(&session.id, &[]);
            assert!(store
                .get(TEST_OWNER, &session.id)
                .unwrap()
                .unwrap()
                .surfaced_entities
                .is_empty());
            session.id
        };

        let reopened = SessionStore::with_db(db)
            .get(TEST_OWNER, &session_id)
            .unwrap()
            .unwrap();
        assert!(reopened.surfaced_entities.is_empty());
    }

    #[test]
    fn manual_panel_toggle_persists() {
        let db = test_db();
        let session_id = {
            let store = SessionStore::with_db(db.clone());
            let session = create_test_session(&store);
            store.set_entity_panel_open(&session.id, true);
            store.set_entity_panel_open(&session.id, false);
            session.id
        };
        let reopened = SessionStore::with_db(db)
            .get(TEST_OWNER, &session_id)
            .unwrap()
            .unwrap();
        assert!(!reopened.entity_panel_open);
    }

    #[test]
    fn owner_switch_hides_other_sessions_and_keeps_shared_legacy_sessions() {
        let db = test_db();
        let store = SessionStore::with_db(db.clone());
        let session_a =
            store.create_session_with_runtime("usr_a", AssistantRuntimeSelection::default());
        let session_b =
            store.create_session_with_runtime("usr_b", AssistantRuntimeSelection::default());
        let shared = store.create_session_with_runtime("", AssistantRuntimeSelection::default());

        let visible_to_a = store
            .list("usr_a")
            .into_iter()
            .map(|session| session.id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            visible_to_a,
            std::collections::HashSet::from([session_a.id.clone(), shared.id.clone()])
        );
        assert!(store.get("usr_a", &session_b.id).unwrap().is_none());
        assert!(store
            .set_runtime("usr_b", &session_a.id, AssistantRuntimeSelection::default(),)
            .unwrap()
            .is_none());

        store.delete("usr_b", &session_a.id);
        assert!(SessionStore::with_db(db)
            .get("usr_a", &session_a.id)
            .unwrap()
            .is_some());
    }

    #[test]
    fn persisted_sessions_load_only_for_the_requested_owner() {
        let db = test_db();
        assistant::assistant_session_upsert(&db, "usr_a", "ses_a", "a", "t0", "t0").unwrap();
        assistant::assistant_session_upsert(&db, "usr_b", "ses_b", "b", "t0", "t0").unwrap();
        assistant::assistant_session_upsert(&db, "", "ses_shared", "shared", "t0", "t0").unwrap();
        let store = SessionStore::with_db(db);

        assert!(store.state.lock().unwrap().sessions.is_empty());

        let visible_to_a = store
            .list("usr_a")
            .into_iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>();

        assert_eq!(
            visible_to_a,
            HashSet::from(["ses_a".to_string(), "ses_shared".to_string()])
        );
        assert!(!store.state.lock().unwrap().sessions.contains_key("ses_b"));

        let visible_to_b = store
            .list("usr_b")
            .into_iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>();

        assert_eq!(
            visible_to_b,
            HashSet::from(["ses_b".to_string(), "ses_shared".to_string()])
        );
    }

    #[test]
    fn is_current_turn_tracks_the_latest_turn() {
        let store = SessionStore::with_db(test_db());
        let session = create_test_session(&store);

        store.set_active_turn(
            &session.id,
            Some(ActiveTurn {
                turn_id: "turn_a".into(),
                status: TurnStatus::Running,
            }),
        );
        assert!(store.is_current_turn(&session.id, "turn_a"));
        assert!(!store.is_current_turn(&session.id, "turn_b"));

        // A newer turn takes over: the superseded one is no longer current.
        store.set_active_turn(
            &session.id,
            Some(ActiveTurn {
                turn_id: "turn_b".into(),
                status: TurnStatus::Running,
            }),
        );
        assert!(!store.is_current_turn(&session.id, "turn_a"));
        assert!(store.is_current_turn(&session.id, "turn_b"));
        assert!(!store.is_current_turn("missing", "turn_b"));
    }
}
