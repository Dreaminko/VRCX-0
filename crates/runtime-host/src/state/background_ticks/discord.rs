use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use serde_json::json;
use vrcx_0_application::{
    build_background_discord_presence_command, build_background_presence_facts,
    BackgroundDiscordPresenceCommand, BackgroundDiscordPresenceState, BackgroundPresenceFactsInput,
    DiscordPresenceLabels,
};
use vrcx_0_host::discord_rpc::DiscordRpc;
use vrcx_0_i18n::{parse_catalog, Catalog};
use vrcx_0_persistence::config::ConfigRepository;

use super::super::{
    background_capability_session, emit_background_error, emit_background_info_if_changed,
    BACKGROUND_DISCORD_CADENCE_SECONDS, BACKGROUND_DISCORD_PRESENCE_JOB,
};
use super::BackgroundTickContext;

const APP_LANGUAGE_CONFIG_KEY: &str = "appLanguage";
const DISCORD_PRESENCE_JSON: &str = include_str!("../discord_presence.json");

fn discord_presence_catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        parse_catalog(
            DISCORD_PRESENCE_JSON,
            "Discord presence localization catalog",
        )
    })
}

fn discord_presence_labels(config: &ConfigRepository) -> DiscordPresenceLabels {
    let language = config
        .get_string(APP_LANGUAGE_CONFIG_KEY, "en")
        .unwrap_or_else(|_| "en".into());
    let catalog = discord_presence_catalog();
    let text = |key: &str, fallback: &str| catalog.text(&language, key, fallback);
    DiscordPresenceLabels {
        access_public: text("discord.access.public", "Public"),
        access_invite_plus: text("discord.access.invite_plus", "Invite+"),
        access_invite: text("discord.access.invite", "Invite"),
        access_friends: text("discord.access.friends", "Friends"),
        access_friends_plus: text("discord.access.friends_plus", "Friends+"),
        access_group: text("discord.access.group", "Group"),
        group_access_public: text("discord.access.group_public", "Public"),
        group_access_plus: text("discord.access.group_plus", "Plus"),
        group_access_members: text("discord.access.group_members", "Members"),
        status_active: text("discord.status.active", "Active"),
        status_join_me: text("discord.status.join_me", "Join Me"),
        status_ask_me: text("discord.status.ask_me", "Ask Me"),
        status_busy: text("discord.status.busy", "Do Not Disturb"),
        status_offline: text("discord.status.offline", "Offline"),
        platform_desktop: text("discord.platform.desktop", "Desktop"),
        platform_vr: text("discord.platform.vr", "VR"),
        private_world: text("discord.private_world", "Private World"),
    }
}

pub(in crate::state) async fn run_background_discord_tick(
    context: &BackgroundTickContext<'_>,
    discord_rpc: &Arc<DiscordRpc>,
    discord_state: &mut BackgroundDiscordPresenceState,
    discord_success_info: &mut Option<String>,
    favorite_friend_groups_by_key: &HashMap<String, Vec<String>>,
) {
    context.background_jobs.mark_running(
        BACKGROUND_DISCORD_PRESENCE_JOB,
        "Running background Discord presence.",
    );
    let Some(session) = background_capability_session(context.session_slot) else {
        context.background_jobs.mark_scheduled(
            BACKGROUND_DISCORD_PRESENCE_JOB,
            "Background Discord presence is waiting for an authenticated session.",
            BACKGROUND_DISCORD_CADENCE_SECONDS,
        );
        return;
    };
    let host_session = context.runtime_context.session.snapshot();
    let friends_by_id = context
        .realtime_runtime
        .friend_snapshot()
        .map(|snapshot| snapshot.friends_by_id)
        .unwrap_or_default();
    let facts = match build_background_presence_facts(
        context.db.as_ref(),
        BackgroundPresenceFactsInput {
            session,
            is_game_running: host_session.is_game_running,
            is_steamvr_running: host_session.is_steamvr_running,
            is_game_no_vr: context
                .runtime_context
                .config()
                .get_bool("isGameNoVR", false)
                .unwrap_or(false),
            last_game_started_at: host_session.last_game_started_at,
            game_log_snapshot: context.runtime_context.game_log_snapshot(),
            now_playing: context.runtime_context.now_playing(),
            friends_by_id,
            favorite_friend_groups_by_key: favorite_friend_groups_by_key.clone(),
        },
    ) {
        Ok(facts) => facts,
        Err(error) => {
            tracing::warn!(error = %error, "background Discord facts build failed");
            *discord_success_info = None;
            emit_background_error(
                context.runtime_context,
                context.backend_runtime,
                format!("Discord presence facts failed: {error}."),
            );
            context
                .background_jobs
                .mark_failed(BACKGROUND_DISCORD_PRESENCE_JOB, error.to_string());
            return;
        }
    };
    let command = match build_background_discord_presence_command(
        context.runtime_context.config(),
        context.web.as_ref(),
        context.db.as_ref(),
        &facts,
        &discord_presence_labels(context.runtime_context.config()),
        discord_state,
        false,
    )
    .await
    {
        Ok(command) => command,
        Err(error) => {
            tracing::warn!(error = %error, "background Discord presence compose failed");
            *discord_success_info = None;
            emit_background_error(
                context.runtime_context,
                context.backend_runtime,
                format!("Discord presence compose failed: {error}."),
            );
            context
                .background_jobs
                .mark_failed(BACKGROUND_DISCORD_PRESENCE_JOB, error.to_string());
            return;
        }
    };

    let detail = match command {
        BackgroundDiscordPresenceCommand::Noop { detail } => detail,
        BackgroundDiscordPresenceCommand::SetActive { active, detail, .. } => {
            let rpc = Arc::clone(discord_rpc);
            match tokio::task::spawn_blocking(move || rpc.set_active(active)).await {
                Ok(Ok(result)) => {
                    discord_state.apply_set_active_result(result);
                    emit_background_info_if_changed(
                        context.runtime_context,
                        context.backend_runtime,
                        discord_success_info,
                        format!(
                            "Discord presence {}: {detail}",
                            if active { "connected" } else { "cleared" }
                        ),
                    );
                    detail
                }
                Ok(Err(error)) => {
                    discord_state.apply_set_active_result(false);
                    tracing::warn!(error = %error, "background Discord SetActive failed");
                    *discord_success_info = None;
                    emit_background_error(
                        context.runtime_context,
                        context.backend_runtime,
                        format!("Discord SetActive failed: {error}."),
                    );
                    context
                        .background_jobs
                        .mark_failed(BACKGROUND_DISCORD_PRESENCE_JOB, error.to_string());
                    return;
                }
                Err(error) => {
                    discord_state.apply_set_active_result(false);
                    tracing::warn!(error = %error, "background Discord SetActive task failed");
                    *discord_success_info = None;
                    emit_background_error(
                        context.runtime_context,
                        context.backend_runtime,
                        format!("Discord SetActive task failed: {error}."),
                    );
                    context
                        .background_jobs
                        .mark_failed(BACKGROUND_DISCORD_PRESENCE_JOB, error.to_string());
                    return;
                }
            }
        }
        BackgroundDiscordPresenceCommand::SetAssets { payload } => {
            let detail = payload.detail.clone();
            let rpc = Arc::clone(discord_rpc);
            let rpc_payload = json!({
                "appId": payload.app_id,
                "activity": payload.activity.clone(),
            });
            match tokio::task::spawn_blocking(move || rpc.set_assets(rpc_payload)).await {
                Ok(Ok(result)) => {
                    discord_state.apply_set_assets_result(&payload, result);
                    emit_background_info_if_changed(
                        context.runtime_context,
                        context.backend_runtime,
                        discord_success_info,
                        format!("Discord activity sent: {detail}"),
                    );
                    detail
                }
                Ok(Err(error)) => {
                    discord_state.apply_set_assets_result(&payload, false);
                    tracing::warn!(error = %error, "background Discord SetAssets failed");
                    *discord_success_info = None;
                    emit_background_error(
                        context.runtime_context,
                        context.backend_runtime,
                        format!("Discord SetAssets failed: {error}."),
                    );
                    context
                        .background_jobs
                        .mark_failed(BACKGROUND_DISCORD_PRESENCE_JOB, error.to_string());
                    return;
                }
                Err(error) => {
                    discord_state.apply_set_assets_result(&payload, false);
                    tracing::warn!(error = %error, "background Discord SetAssets task failed");
                    *discord_success_info = None;
                    emit_background_error(
                        context.runtime_context,
                        context.backend_runtime,
                        format!("Discord SetAssets task failed: {error}."),
                    );
                    context
                        .background_jobs
                        .mark_failed(BACKGROUND_DISCORD_PRESENCE_JOB, error.to_string());
                    return;
                }
            }
        }
    };
    context
        .background_jobs
        .mark_completed(BACKGROUND_DISCORD_PRESENCE_JOB, detail);
    context.background_jobs.mark_scheduled(
        BACKGROUND_DISCORD_PRESENCE_JOB,
        "Next background Discord presence tick is waiting.",
        BACKGROUND_DISCORD_CADENCE_SECONDS,
    );
}
