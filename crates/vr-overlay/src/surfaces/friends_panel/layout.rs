use crate::{
    layout::{ellipsize_to_width, TextMeasurer},
    model::{Color, OverlaySize, OverlaySurfaceId, Rect},
    scene::{DrawCommand, HitRegion, OverlayScene, TextStyle},
};

use super::{
    model::{
        visible_category_count, visible_row_count, FavoriteFriendsPanelModel, FriendPanelRow,
        FriendPanelRowPrimaryAction, FriendPanelStatusTone, FRIENDS_PANEL_SURFACE_ID,
    },
    style,
};

const ACTION_BUTTON_WIDTH: f32 = 86.0;
const ACTION_BUTTON_HEIGHT: f32 = 30.0;
const ACTION_BUTTON_GAP: f32 = 8.0;
const ACTION_COLUMN_RESERVED: f32 = 110.0;
const ROW_SCROLLBAR_WIDTH: f32 = 18.0;
const ROW_SCROLLBAR_GAP: f32 = 10.0;
const ROW_SCROLLBAR_PADDING: f32 = 8.0;
const ROW_SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 36.0;

#[derive(Clone, Copy)]
enum RowActionButtonKind {
    Open,
    Request,
    Invite,
}

impl RowActionButtonKind {
    fn key(self) -> &'static str {
        match self {
            RowActionButtonKind::Open => "open",
            RowActionButtonKind::Request => "request",
            RowActionButtonKind::Invite => "invite",
        }
    }

    fn label(self, model: &FavoriteFriendsPanelModel) -> &str {
        match self {
            RowActionButtonKind::Open => &model.strings.open_label,
            RowActionButtonKind::Request => &model.strings.request_label,
            RowActionButtonKind::Invite => &model.strings.invite_label,
        }
    }

    fn accent(self) -> Color {
        match self {
            RowActionButtonKind::Open => style::ACCENT,
            RowActionButtonKind::Request => style::ASK_ME,
            RowActionButtonKind::Invite => style::FAVORITE,
        }
    }
}

pub fn build_friends_panel_scene(model: &FavoriteFriendsPanelModel) -> OverlayScene {
    let mut text = TextMeasurer::new();
    build_friends_panel_scene_with_text(model, &mut text)
}

pub fn build_friends_panel_scene_with_text(
    model: &FavoriteFriendsPanelModel,
    text: &mut TextMeasurer,
) -> OverlayScene {
    let mut scene = OverlayScene::new(OverlaySurfaceId::new(FRIENDS_PANEL_SURFACE_ID), model.size);
    let width = model.size.width as f32;
    let height = model.size.height as f32;

    scene.push(DrawCommand::FillRect {
        rect: Rect::new(0.0, 0.0, width, height),
        color: style::BACKGROUND,
    });
    scene.push(DrawCommand::FillRect {
        rect: Rect::new(
            style::MARGIN,
            style::MARGIN,
            width - style::MARGIN * 2.0,
            height - style::MARGIN * 2.0,
        ),
        color: style::PANEL,
    });
    scene.push(DrawCommand::Text {
        origin_x: style::MARGIN + 22.0,
        origin_y: style::HEADER_Y,
        max_width: width - style::MARGIN * 2.0 - 44.0,
        text: model.strings.title.clone(),
        style: TextStyle::new(32.0, 38.0, style::TEXT),
    });
    let categories_rect = category_list_rect();
    let list_rect = list_rect(model.size);
    push_panel_list_frame(&mut scene, categories_rect, "category-list", model);
    push_panel_list_frame(&mut scene, list_rect, "list", model);
    push_categories(&mut scene, text, model, categories_rect);

    if model.rows.is_empty() {
        scene.push(DrawCommand::Text {
            origin_x: list_rect.x + 28.0,
            origin_y: list_rect.y + 42.0,
            max_width: list_rect.width - 56.0,
            text: model.strings.empty_label.clone(),
            style: TextStyle::new(24.0, 30.0, style::MUTED),
        });
    } else {
        push_rows(&mut scene, text, model, list_rect);
    }
    push_list_overflow_masks(&mut scene, list_rect, model.size);
    if let Some(message) = model
        .status_message
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        scene.push(DrawCommand::Text {
            origin_x: style::MARGIN + 22.0,
            origin_y: style::HEADER_Y + 44.0,
            max_width: width - style::MARGIN * 2.0 - 44.0,
            text: ellipsize_to_width(
                text,
                message.trim(),
                width - style::MARGIN * 2.0 - 44.0,
                20.0,
            ),
            style: TextStyle::new(20.0, 26.0, style::ACCENT),
        });
    }
    push_row_scrollbar(&mut scene, model, list_rect);
    scene.push(DrawCommand::StrokeRect {
        rect: list_rect,
        color: if model.hovered_region_id.as_deref() == Some("list") {
            style::ACCENT
        } else {
            style::DIVIDER
        },
        width: 2.0,
    });

    scene.hit_regions = friends_panel_hit_regions(model);
    scene
}

pub fn friends_panel_hit_regions(model: &FavoriteFriendsPanelModel) -> Vec<HitRegion> {
    let mut regions = Vec::new();
    let category_list = category_list_rect();
    for (visible_index, (_, category)) in model.visible_categories().enumerate() {
        regions.push(HitRegion {
            id: format!("cat:{}", category.key),
            rect: Rect::new(
                category_list.x,
                category_list.y + visible_index as f32 * style::CATEGORY_HEIGHT,
                category_list.width,
                style::CATEGORY_HEIGHT,
            ),
        });
    }
    regions.push(HitRegion {
        id: "category-list".to_string(),
        rect: category_list,
    });
    let list = list_rect(model.size);
    if let Some(thumb) = row_scrollbar_thumb_rect(model, list) {
        regions.push(HitRegion {
            id: "scroll-thumb".to_string(),
            rect: thumb,
        });
        regions.push(HitRegion {
            id: "scroll-track".to_string(),
            rect: row_scrollbar_track_rect(list),
        });
    }
    let row_content = row_content_rect(list);
    for (visible_index, (_, row)) in model.visible_rows().enumerate() {
        let row_rect = Rect::new(
            list.x,
            list.y + visible_index as f32 * style::ROW_HEIGHT
                - model.row_scroll_fraction() * style::ROW_HEIGHT,
            row_content.width,
            style::ROW_HEIGHT,
        );
        let Some(row_region_rect) = clipped_rect(row_rect, row_content) else {
            continue;
        };
        if row_is_section_header(row) {
            continue;
        }
        let buttons = row_action_button_kinds(row);
        for (button_index, button) in buttons.iter().enumerate() {
            let button_rect = row_action_button_rect(row_rect, buttons.len(), button_index);
            let Some(button_rect) = clipped_rect(button_rect, row_content) else {
                continue;
            };
            regions.push(HitRegion {
                id: row_action_button_id(&row.user_id, *button),
                rect: button_rect,
            });
        }
        regions.push(HitRegion {
            id: format!("row:{}", row.user_id),
            rect: row_region_rect,
        });
    }
    regions.push(HitRegion {
        id: "list".to_string(),
        rect: row_content,
    });
    regions
}

fn push_panel_list_frame(
    scene: &mut OverlayScene,
    rect: Rect,
    hover_region_id: &str,
    model: &FavoriteFriendsPanelModel,
) {
    scene.push(DrawCommand::FillRect {
        rect,
        color: style::PANEL_ALT,
    });
    scene.push(DrawCommand::StrokeRect {
        rect,
        color: if model.hovered_region_id.as_deref() == Some(hover_region_id) {
            style::ACCENT
        } else {
            style::DIVIDER
        },
        width: 2.0,
    });
}

fn push_categories(
    scene: &mut OverlayScene,
    text: &mut TextMeasurer,
    model: &FavoriteFriendsPanelModel,
    list_rect: Rect,
) {
    for (visible_index, (_, category)) in model.visible_categories().enumerate() {
        let rect = Rect::new(
            list_rect.x,
            list_rect.y + visible_index as f32 * style::CATEGORY_HEIGHT,
            list_rect.width,
            style::CATEGORY_HEIGHT,
        );
        let id = format!("cat:{}", category.key);
        let selected = model.selected_category_key == category.key;
        let hovered = model.hovered_region_id.as_deref() == Some(id.as_str());
        let pressed = model.pressed_region_id.as_deref() == Some(id.as_str());
        let fill = if pressed {
            style::PANEL_PRESSED
        } else if selected {
            Color::rgba(37, 99, 110, 255)
        } else if hovered {
            style::PANEL_HOVER
        } else {
            style::PANEL_ALT
        };
        scene.push(DrawCommand::FillRect { rect, color: fill });
        scene.push(DrawCommand::StrokeRect {
            rect,
            color: if selected || hovered || pressed {
                style::ACCENT
            } else {
                style::DIVIDER
            },
            width: 2.0,
        });
        let max_width = rect.width - 32.0;
        scene.push(DrawCommand::Text {
            origin_x: rect.x + 16.0,
            origin_y: rect.y + 14.0,
            max_width,
            text: ellipsize_to_width(
                text,
                &format!("{} {}", category.label, category.count),
                max_width,
                19.0,
            ),
            style: TextStyle::new(
                19.0,
                24.0,
                if selected { style::TEXT } else { style::MUTED },
            ),
        });
    }
}

fn push_rows(
    scene: &mut OverlayScene,
    text: &mut TextMeasurer,
    model: &FavoriteFriendsPanelModel,
    list_rect: Rect,
) {
    let row_content = row_content_rect(list_rect);
    for (visible_index, (_, row)) in model.visible_rows().enumerate() {
        let rect = Rect::new(
            row_content.x,
            list_rect.y + visible_index as f32 * style::ROW_HEIGHT
                - model.row_scroll_fraction() * style::ROW_HEIGHT,
            row_content.width,
            style::ROW_HEIGHT,
        );
        if rect.y + rect.height <= list_rect.y || rect.y >= list_rect.y + list_rect.height {
            continue;
        }
        let id = format!("row:{}", row.user_id);
        let hovered = model.hovered_region_id.as_deref() == Some(id.as_str());
        let pressed = model.pressed_region_id.as_deref() == Some(id.as_str());
        let fill = if pressed {
            Color::rgba(23, 93, 112, 255)
        } else if hovered {
            style::PANEL_HOVER
        } else if visible_index % 2 == 0 {
            style::PANEL_ALT
        } else {
            Color::rgba(21, 31, 43, 248)
        };
        scene.push(DrawCommand::FillRect { rect, color: fill });
        if row_is_section_header(row) {
            push_section_header(scene, text, row, rect);
        } else {
            push_row_contents(scene, text, model, row, rect);
        }
        if visible_index + 1 < visible_row_count() + 1 {
            scene.push(DrawCommand::FillRect {
                rect: Rect::new(
                    rect.x + 18.0,
                    rect.y + rect.height - 1.0,
                    rect.width - 36.0,
                    1.0,
                ),
                color: style::DIVIDER,
            });
        }
    }
}

fn push_section_header(
    scene: &mut OverlayScene,
    text: &mut TextMeasurer,
    row: &FriendPanelRow,
    rect: Rect,
) {
    let Some(label) = row.section_label.as_deref() else {
        return;
    };
    let max_width = rect.width - 52.0;
    scene.push(DrawCommand::FillRect {
        rect: Rect::new(rect.x + 24.0, rect.y + 42.0, rect.width - 48.0, 2.0),
        color: style::ACCENT,
    });
    scene.push(DrawCommand::Text {
        origin_x: rect.x + 28.0,
        origin_y: rect.y + 50.0,
        max_width,
        text: ellipsize_to_width(text, label, max_width, 22.0),
        style: TextStyle::new(22.0, 28.0, style::TEXT),
    });
}

fn push_row_contents(
    scene: &mut OverlayScene,
    text: &mut TextMeasurer,
    model: &FavoriteFriendsPanelModel,
    row: &FriendPanelRow,
    rect: Rect,
) {
    push_row_action_buttons(scene, text, model, row, rect);

    let avatar_x = rect.x + 22.0 + ACTION_COLUMN_RESERVED;
    let avatar_y = rect.y + 17.0;
    if let Some(avatar) = &row.avatar {
        scene.push(DrawCommand::Image {
            rect: Rect::new(avatar_x, avatar_y, style::AVATAR_SIZE, style::AVATAR_SIZE),
            rgba: avatar.rgba.clone(),
            width: avatar.width,
            height: avatar.height,
        });
    } else {
        scene.push(DrawCommand::Circle {
            center_x: avatar_x + style::AVATAR_SIZE * 0.5,
            center_y: avatar_y + style::AVATAR_SIZE * 0.5,
            radius: style::AVATAR_SIZE * 0.5,
            color: Color::rgba(51, 65, 85, 255),
        });
    }

    scene.push(DrawCommand::Circle {
        center_x: avatar_x + style::AVATAR_SIZE - 8.0,
        center_y: avatar_y + style::AVATAR_SIZE - 8.0,
        radius: 8.0,
        color: status_color(row.status),
    });

    let text_x = avatar_x + style::AVATAR_SIZE + 22.0;
    let right_reserved = if row.is_traveling { 92.0 } else { 28.0 };
    let text_width = (rect.x + rect.width - right_reserved - text_x).max(1.0);
    scene.push(DrawCommand::Text {
        origin_x: text_x,
        origin_y: rect.y + 12.0,
        max_width: text_width,
        text: ellipsize_to_width(text, &row.display_name, text_width, 26.0),
        style: TextStyle::new(26.0, 32.0, style::FAVORITE),
    });
    let location = location_line(row);
    scene.push(DrawCommand::Text {
        origin_x: text_x,
        origin_y: rect.y + 43.0,
        max_width: text_width,
        text: ellipsize_to_width(text, &location, text_width, 20.0),
        style: TextStyle::new(20.0, 26.0, style::TEXT),
    });
    let note = row.note.as_deref().filter(|value| !value.trim().is_empty());
    let memo = row.memo.as_deref().filter(|value| !value.trim().is_empty());
    let has_note_and_memo = note.is_some() && memo.is_some();
    if let Some(note) = note {
        let max_width = if has_note_and_memo {
            text_width * 0.49
        } else {
            text_width
        };
        let note_text = format!("{}: {note}", model.strings.note_label);
        scene.push(DrawCommand::Text {
            origin_x: text_x,
            origin_y: rect.y + 68.0,
            max_width,
            text: ellipsize_to_width(text, &note_text, max_width, 16.0),
            style: TextStyle::new(16.0, 22.0, style::MUTED),
        });
    }
    if let Some(memo) = memo {
        let origin_x = if has_note_and_memo {
            text_x + text_width * 0.51
        } else {
            text_x
        };
        let max_width = if has_note_and_memo {
            text_width * 0.49
        } else {
            text_width
        };
        let memo_text = format!("{}: {memo}", model.strings.memo_label);
        scene.push(DrawCommand::Text {
            origin_x,
            origin_y: rect.y + 68.0,
            max_width,
            text: ellipsize_to_width(text, &memo_text, max_width, 16.0),
            style: TextStyle::new(16.0, 22.0, style::SUBTLE),
        });
    }

    if row.is_traveling {
        push_spinner(
            scene,
            rect.x + rect.width - 48.0,
            rect.y + rect.height * 0.5,
            model.spinner_phase,
        );
    }
}

fn push_row_action_buttons(
    scene: &mut OverlayScene,
    text: &mut TextMeasurer,
    model: &FavoriteFriendsPanelModel,
    row: &FriendPanelRow,
    rect: Rect,
) {
    let buttons = row_action_button_kinds(row);
    let count = buttons.len();
    for (index, button) in buttons.into_iter().enumerate() {
        let button_rect = row_action_button_rect(rect, count, index);
        let id = row_action_button_id(&row.user_id, button);
        let hovered = model.hovered_region_id.as_deref() == Some(id.as_str());
        let pressed = model.pressed_region_id.as_deref() == Some(id.as_str());
        let armed = model.armed_action_region_id.as_deref() == Some(id.as_str());
        let accent = button.accent();
        let fill = if pressed {
            style::PANEL_PRESSED
        } else if armed {
            Color::rgba(accent.r, accent.g, accent.b, 72)
        } else if hovered {
            style::PANEL_HOVER
        } else {
            Color::rgba(15, 23, 42, 255)
        };
        scene.push(DrawCommand::FillRect {
            rect: button_rect,
            color: fill,
        });
        scene.push(DrawCommand::StrokeRect {
            rect: button_rect,
            color: if armed { style::TEXT } else { accent },
            width: if armed { 3.0 } else { 2.0 },
        });
        let label = button.label(model);
        let max_width = button_rect.width - 12.0;
        scene.push(DrawCommand::Text {
            origin_x: button_rect.x + 6.0,
            origin_y: button_rect.y + 6.0,
            max_width,
            text: ellipsize_to_width(text, label, max_width, 15.0),
            style: TextStyle::new(15.0, 18.0, if armed { accent } else { style::TEXT }),
        });
    }
}

fn row_action_button_kinds(row: &FriendPanelRow) -> Vec<RowActionButtonKind> {
    let mut buttons = Vec::new();
    match row.actions.primary {
        Some(FriendPanelRowPrimaryAction::Open) => buttons.push(RowActionButtonKind::Open),
        Some(FriendPanelRowPrimaryAction::Request) => buttons.push(RowActionButtonKind::Request),
        None => {}
    }
    if row.actions.invite {
        buttons.push(RowActionButtonKind::Invite);
    }
    buttons
}

fn row_is_section_header(row: &FriendPanelRow) -> bool {
    row.section_label.is_some() && row.user_id.is_empty()
}

fn row_action_button_rect(row_rect: Rect, count: usize, index: usize) -> Rect {
    let total_height =
        count as f32 * ACTION_BUTTON_HEIGHT + count.saturating_sub(1) as f32 * ACTION_BUTTON_GAP;
    let start_y = row_rect.y + (row_rect.height - total_height).max(0.0) * 0.5;
    Rect::new(
        row_rect.x + 16.0,
        start_y + index as f32 * (ACTION_BUTTON_HEIGHT + ACTION_BUTTON_GAP),
        ACTION_BUTTON_WIDTH,
        ACTION_BUTTON_HEIGHT,
    )
}

fn row_action_button_id(user_id: &str, button: RowActionButtonKind) -> String {
    format!("action:{user_id}:{}", button.key())
}

fn location_line(row: &FriendPanelRow) -> String {
    if row.is_traveling {
        return row
            .traveling_text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|target| format!("{} -> {target}", row.location_text))
            .unwrap_or_else(|| row.location_text.clone());
    }
    row.location_text.clone()
}

fn push_spinner(scene: &mut OverlayScene, center_x: f32, center_y: f32, phase: f32) {
    let phase = phase.rem_euclid(1.0);
    for index in 0..8 {
        let angle = (index as f32 / 8.0 + phase) * std::f32::consts::TAU;
        let alpha = 70 + ((index as f32 / 7.0) * 185.0).round() as u8;
        scene.push(DrawCommand::Circle {
            center_x: center_x + angle.cos() * 16.0,
            center_y: center_y + angle.sin() * 16.0,
            radius: 4.0,
            color: Color::rgba(style::ACCENT.r, style::ACCENT.g, style::ACCENT.b, alpha),
        });
    }
}

fn status_color(status: FriendPanelStatusTone) -> Color {
    match status {
        FriendPanelStatusTone::Online => style::ONLINE,
        FriendPanelStatusTone::Active => style::ACTIVE,
        FriendPanelStatusTone::Busy => style::BUSY,
        FriendPanelStatusTone::AskMe => style::ASK_ME,
        FriendPanelStatusTone::Offline => style::OFFLINE,
    }
}

fn list_rect(size: OverlaySize) -> Rect {
    let width = size.width as f32;
    let categories = category_list_rect();
    let x = categories.x + categories.width + style::CATEGORY_GAP;
    Rect::new(
        x,
        style::LIST_Y,
        width - x - style::MARGIN - 22.0,
        style::ROW_HEIGHT * visible_row_count() as f32,
    )
}

fn category_list_rect() -> Rect {
    Rect::new(
        style::MARGIN + 22.0,
        style::LIST_Y,
        style::CATEGORY_WIDTH,
        style::CATEGORY_HEIGHT * visible_category_count() as f32,
    )
}

fn row_content_rect(list: Rect) -> Rect {
    Rect::new(
        list.x,
        list.y,
        list.width - ROW_SCROLLBAR_WIDTH - ROW_SCROLLBAR_GAP,
        list.height,
    )
}

fn row_scrollbar_track_rect(list: Rect) -> Rect {
    Rect::new(
        list.x + list.width - ROW_SCROLLBAR_WIDTH - ROW_SCROLLBAR_PADDING,
        list.y + ROW_SCROLLBAR_PADDING,
        ROW_SCROLLBAR_WIDTH,
        list.height - ROW_SCROLLBAR_PADDING * 2.0,
    )
}

fn row_scrollbar_thumb_rect(model: &FavoriteFriendsPanelModel, list: Rect) -> Option<Rect> {
    let max = model.max_row_scroll_offset();
    if max <= 0.0 {
        return None;
    }
    let track = row_scrollbar_track_rect(list);
    let visible = visible_row_count() as f32;
    let total = model.rows.len().max(visible_row_count()) as f32;
    let thumb_height =
        (track.height * visible / total).clamp(ROW_SCROLLBAR_MIN_THUMB_HEIGHT, track.height);
    let travel = (track.height - thumb_height).max(0.0);
    let y = track.y + travel * (model.row_scroll_offset / max).clamp(0.0, 1.0);
    Some(Rect::new(track.x, y, track.width, thumb_height))
}

fn push_row_scrollbar(
    scene: &mut OverlayScene,
    model: &FavoriteFriendsPanelModel,
    list_rect: Rect,
) {
    let Some(thumb) = row_scrollbar_thumb_rect(model, list_rect) else {
        return;
    };
    let track = row_scrollbar_track_rect(list_rect);
    let hovered_thumb = model.hovered_region_id.as_deref() == Some("scroll-thumb");
    let pressed_thumb = model.pressed_region_id.as_deref() == Some("scroll-thumb");
    scene.push(DrawCommand::FillRect {
        rect: track,
        color: Color::rgba(15, 23, 42, 220),
    });
    scene.push(DrawCommand::FillRect {
        rect: thumb,
        color: if pressed_thumb {
            style::PANEL_PRESSED
        } else if hovered_thumb {
            style::ACCENT
        } else {
            style::MUTED
        },
    });
}

fn push_list_overflow_masks(scene: &mut OverlayScene, list_rect: Rect, size: OverlaySize) {
    let top = style::HEADER_Y + 48.0;
    if list_rect.y > top {
        scene.push(DrawCommand::FillRect {
            rect: Rect::new(list_rect.x, top, list_rect.width, list_rect.y - top),
            color: style::PANEL,
        });
    }
    let bottom = list_rect.y + list_rect.height;
    let panel_bottom = size.height as f32 - style::MARGIN;
    if panel_bottom > bottom {
        scene.push(DrawCommand::FillRect {
            rect: Rect::new(list_rect.x, bottom, list_rect.width, panel_bottom - bottom),
            color: style::PANEL,
        });
    }
}

fn clipped_rect(rect: Rect, bounds: Rect) -> Option<Rect> {
    let x = rect.x.max(bounds.x);
    let y = rect.y.max(bounds.y);
    let right = (rect.x + rect.width).min(bounds.x + bounds.width);
    let bottom = (rect.y + rect.height).min(bounds.y + bounds.height);
    (right > x && bottom > y).then(|| Rect::new(x, y, right - x, bottom - y))
}
