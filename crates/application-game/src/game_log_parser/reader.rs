use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use chrono::{Local, NaiveDateTime};
use vrcx_0_core::game_log_parser::parse_log_line_header;

use super::context::LogContext;
use super::media::{
    parse_api_request, parse_avatar_change, parse_avatar_pedestal_change, parse_avpro_video_change,
    parse_join_blocked, parse_screenshot, parse_sdk2_video_play, parse_usharp_video_play,
    parse_usharp_video_sync, parse_video_change, parse_video_error, parse_world_vrcx,
};
use super::presence::{
    parse_location, parse_location_destination, parse_notification, parse_player_joined_or_left,
    parse_portal_spawn,
};
use super::sink::GameLogParseSink;
use super::system::{
    parse_application_quit, parse_audio_config, parse_desktop_mode, parse_failed_to_join,
    parse_image_download, parse_instance_reset, parse_openvr_init, parse_osc_failed,
    parse_shader_keywords_limit, parse_sticker_spawn, parse_string_download, parse_udon_exception,
    parse_untrusted_url, parse_vote_kick, parse_vote_kick_init, parse_vote_kick_success,
};

pub(crate) fn parse_log(
    out: &mut dyn GameLogParseSink,
    path: &Path,
    file_name: &str,
    ctx: &mut LogContext,
    till_date: NaiveDateTime,
) -> bool {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut reader = BufReader::with_capacity(65536, file);
    if reader.seek(SeekFrom::Start(ctx.position)).is_err() {
        return false;
    }

    let mut line = String::new();
    let initial_position = ctx.position;
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Err(_) => break,
            _ => {}
        }

        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }

        if parse_udon_exception(out, file_name, trimmed) {
            continue;
        }

        let Some((line_date, content)) = parse_log_line_header(trimmed) else {
            continue;
        };

        if line_date <= till_date {
            continue;
        }

        let now_local = Local::now().naive_local();
        if line_date > now_local + chrono::Duration::minutes(61) {
            continue;
        }

        if content.starts_with('[') {
            let _ = parse_player_joined_or_left(out, file_name, trimmed, content)
                || parse_location(out, file_name, trimmed, content, ctx)
                || parse_location_destination(out, file_name, trimmed, content, ctx)
                || parse_portal_spawn(out, file_name, trimmed)
                || parse_notification(out, file_name, trimmed, content)
                || parse_api_request(out, file_name, trimmed, content)
                || parse_avatar_change(out, file_name, trimmed, content)
                || parse_join_blocked(out, file_name, trimmed, content)
                || parse_avatar_pedestal_change(out, file_name, trimmed, content)
                || parse_video_error(out, file_name, trimmed, content, ctx)
                || parse_video_change(out, file_name, trimmed, content)
                || parse_avpro_video_change(out, file_name, trimmed, content)
                || parse_usharp_video_play(out, file_name, trimmed, content)
                || parse_usharp_video_sync(out, file_name, trimmed, content)
                || parse_world_vrcx(out, file_name, trimmed, content)
                || parse_audio_config(out, file_name, trimmed, content, ctx)
                || parse_screenshot(out, file_name, trimmed, content)
                || parse_string_download(out, file_name, trimmed, content)
                || parse_image_download(out, file_name, trimmed, content)
                || parse_vote_kick(out, file_name, trimmed, content)
                || parse_failed_to_join(out, file_name, trimmed, content)
                || parse_instance_reset(out, file_name, trimmed, content)
                || parse_vote_kick_init(out, file_name, trimmed, content)
                || parse_vote_kick_success(out, file_name, trimmed, content)
                || parse_sticker_spawn(out, file_name, trimmed, content);
        } else {
            let _ = parse_shader_keywords_limit(out, file_name, trimmed, content, ctx)
                || parse_sdk2_video_play(out, file_name, trimmed, content)
                || parse_application_quit(out, file_name, trimmed, content)
                || parse_openvr_init(out, file_name, trimmed, content)
                || parse_desktop_mode(out, file_name, trimmed, content)
                || parse_osc_failed(out, file_name, trimmed, content)
                || parse_untrusted_url(out, file_name, trimmed, content, ctx);
        }
    }

    ctx.position = reader.stream_position().unwrap_or(ctx.position);
    ctx.position > initial_position
}
