use crate::{
    model::{OverlaySize, UvPoint},
    scene::HitRegion,
};

use super::{layout::friends_panel_hit_regions, style};
use crate::surfaces::main::AvatarBitmap;

const ROW_SCROLLBAR_PADDING: f32 = 8.0;
const ROW_SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 36.0;

pub const FRIENDS_PANEL_ID: &str = "friends";
pub const LEGACY_DUMMY_PANEL_ID: &str = "dummy";
pub const FRIENDS_PANEL_SURFACE_ID: &str = "friends-panel";
pub const FRIENDS_PANEL_LASER_LEFT_SURFACE_ID: &str = "friends-panel-laser-left";
pub const FRIENDS_PANEL_LASER_RIGHT_SURFACE_ID: &str = "friends-panel-laser-right";

#[derive(Clone, Debug, PartialEq)]
pub enum FriendPanelAction {
    Hover,
    ClickDown,
    ClickUp,
    Scroll { delta: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FriendPanelScrollDrag {
    pub start_uv_y: f32,
    pub start_row_scroll_offset: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FriendPanelStatusTone {
    Online,
    Active,
    Busy,
    AskMe,
    Offline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FriendPanelCategory {
    pub key: String,
    pub label: String,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FriendPanelRowPrimaryAction {
    Open,
    Request,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FriendPanelRowActions {
    pub primary: Option<FriendPanelRowPrimaryAction>,
    pub invite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FriendPanelRow {
    pub section_label: Option<String>,
    pub user_id: String,
    pub display_name: String,
    pub status: FriendPanelStatusTone,
    pub location_text: String,
    pub is_traveling: bool,
    pub traveling_text: Option<String>,
    pub note: Option<String>,
    pub memo: Option<String>,
    pub avatar: Option<AvatarBitmap>,
    pub actions: FriendPanelRowActions,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FriendPanelStrings {
    pub title: String,
    pub all_label: String,
    pub empty_label: String,
    pub note_label: String,
    pub memo_label: String,
    pub open_label: String,
    pub request_label: String,
    pub invite_label: String,
}

impl Default for FriendPanelStrings {
    fn default() -> Self {
        Self {
            title: "Favorite Friends".to_string(),
            all_label: "All".to_string(),
            empty_label: "No favorite friends online".to_string(),
            note_label: "Note".to_string(),
            memo_label: "Local Note".to_string(),
            open_label: "Open".to_string(),
            request_label: "Request".to_string(),
            invite_label: "Invite".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FavoriteFriendsPanelModel {
    pub size: OverlaySize,
    pub categories: Vec<FriendPanelCategory>,
    pub selected_category_key: String,
    pub rows: Vec<FriendPanelRow>,
    pub hovered_region_id: Option<String>,
    pub pressed_region_id: Option<String>,
    pub armed_action_region_id: Option<String>,
    pub pointer_uv: Option<UvPoint>,
    pub category_scroll_offset: usize,
    pub row_scroll_offset: f32,
    pub row_scroll_drag: Option<FriendPanelScrollDrag>,
    pub spinner_phase: f32,
    pub status_message: Option<String>,
    pub strings: FriendPanelStrings,
}

impl Default for FavoriteFriendsPanelModel {
    fn default() -> Self {
        let strings = FriendPanelStrings::default();
        Self {
            size: OverlaySize::new(1080, 720),
            categories: vec![FriendPanelCategory {
                key: "all".to_string(),
                label: strings.all_label.clone(),
                count: 0,
            }],
            selected_category_key: "all".to_string(),
            rows: Vec::new(),
            hovered_region_id: None,
            pressed_region_id: None,
            armed_action_region_id: None,
            pointer_uv: None,
            category_scroll_offset: 0,
            row_scroll_offset: 0.0,
            row_scroll_drag: None,
            spinner_phase: 0.0,
            status_message: None,
            strings,
        }
    }
}

impl FavoriteFriendsPanelModel {
    pub fn apply_uv_action(&mut self, uv: UvPoint, action: FriendPanelAction) -> Option<String> {
        match action {
            FriendPanelAction::Hover => {
                let hit = self.hit_region_at(uv).map(|region| region.id);
                if self.pressed_region_id.as_deref() == Some("scroll-thumb") {
                    self.drag_row_scroll_to(uv);
                } else if self
                    .armed_action_region_id
                    .as_deref()
                    .is_some_and(|armed| hit.as_deref() != Some(armed))
                {
                    self.armed_action_region_id = None;
                }
                self.pointer_uv = hit.as_ref().map(|_| uv);
                self.hovered_region_id = hit.clone();
                hit
            }
            FriendPanelAction::ClickDown => {
                let hit = self.hit_region_at(uv).map(|region| region.id);
                if hit.as_deref() == Some("scroll-thumb") {
                    self.row_scroll_drag = Some(FriendPanelScrollDrag {
                        start_uv_y: uv.y,
                        start_row_scroll_offset: self.row_scroll_offset,
                    });
                } else {
                    self.row_scroll_drag = None;
                }
                self.pressed_region_id = hit.clone();
                hit
            }
            FriendPanelAction::ClickUp => {
                let hit = self.hit_region_at(uv).map(|region| region.id);
                let hit_id = hit.as_deref();
                if self.pressed_region_id == hit {
                    if let Some(category_key) = hit_id
                        .and_then(|id| id.strip_prefix("cat:"))
                        .map(str::to_string)
                    {
                        if self
                            .categories
                            .iter()
                            .any(|category| category.key == category_key)
                        {
                            self.selected_category_key = category_key;
                            self.row_scroll_offset = 0.0;
                            self.armed_action_region_id = None;
                        }
                    } else if hit_id == Some("scroll-track") {
                        self.page_row_scroll_toward(uv);
                        self.armed_action_region_id = None;
                    } else if let Some(action_id) =
                        hit_id.filter(|id| is_action_region(id)).map(str::to_string)
                    {
                        if self.armed_action_region_id.as_deref() == Some(action_id.as_str()) {
                            self.armed_action_region_id = None;
                        } else {
                            self.armed_action_region_id = Some(action_id);
                        }
                    } else if hit_id != self.armed_action_region_id.as_deref() {
                        self.armed_action_region_id = None;
                    }
                } else if hit_id != self.armed_action_region_id.as_deref() {
                    self.armed_action_region_id = None;
                }
                self.pressed_region_id = None;
                self.row_scroll_drag = None;
                hit
            }
            FriendPanelAction::Scroll { delta } => {
                let hit = self.hit_region_at(uv).map(|region| region.id);
                if hit.as_deref().is_some_and(is_category_region) {
                    let max = self.max_category_scroll_offset() as i32;
                    let next = self.category_scroll_offset as i32 + delta.round() as i32;
                    self.category_scroll_offset = next.clamp(0, max) as usize;
                } else if hit.as_deref().is_some_and(is_row_region) {
                    self.set_row_scroll_offset(self.row_scroll_offset + delta);
                }
                self.armed_action_region_id = None;
                hit
            }
        }
    }

    pub fn max_row_scroll_offset(&self) -> f32 {
        self.rows.len().saturating_sub(visible_row_count()) as f32
    }

    pub fn max_scroll_offset_rows(&self) -> f32 {
        self.max_row_scroll_offset()
    }

    pub fn max_category_scroll_offset(&self) -> usize {
        self.categories
            .len()
            .saturating_sub(visible_category_count())
    }

    pub fn visible_categories(&self) -> impl Iterator<Item = (usize, &FriendPanelCategory)> {
        let start = self.category_scroll_offset.min(self.categories.len());
        self.categories
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_category_count())
    }

    pub fn visible_rows(&self) -> impl Iterator<Item = (usize, &FriendPanelRow)> {
        let start = self
            .row_scroll_offset
            .floor()
            .clamp(0.0, self.rows.len() as f32) as usize;
        let take =
            visible_row_count() + usize::from(self.row_scroll_offset.fract().abs() > f32::EPSILON);
        self.rows.iter().enumerate().skip(start).take(take)
    }

    pub fn has_visible_traveling_row(&self) -> bool {
        self.visible_rows().any(|(_, row)| row.is_traveling)
    }

    pub fn row_scroll_fraction(&self) -> f32 {
        self.row_scroll_offset.fract().max(0.0)
    }

    pub fn disarm_action(&mut self) {
        self.armed_action_region_id = None;
    }

    fn set_row_scroll_offset(&mut self, value: f32) {
        self.row_scroll_offset = value.clamp(0.0, self.max_row_scroll_offset());
    }

    fn drag_row_scroll_to(&mut self, uv: UvPoint) {
        let Some(drag) = &self.row_scroll_drag else {
            return;
        };
        let max = self.max_row_scroll_offset();
        if max <= 0.0 {
            self.row_scroll_offset = 0.0;
            return;
        }
        let track_span = row_scrollbar_track_span();
        let uv_delta = uv.y - drag.start_uv_y;
        let row_delta = uv_delta * self.size.height as f32 / track_span * max;
        self.set_row_scroll_offset(drag.start_row_scroll_offset + row_delta);
    }

    fn page_row_scroll_toward(&mut self, uv: UvPoint) {
        let thumb_center_y = row_scrollbar_thumb_center_y(self);
        let direction = if uv.y * (self.size.height as f32) < thumb_center_y {
            -1.0
        } else {
            1.0
        };
        self.set_row_scroll_offset(self.row_scroll_offset + direction * visible_row_count() as f32);
    }

    fn hit_region_at(&self, uv: UvPoint) -> Option<HitRegion> {
        friends_panel_hit_regions(self)
            .into_iter()
            .find(|region| region.contains_uv(self.size, uv))
    }
}

pub const fn visible_row_count() -> usize {
    style::VISIBLE_ROWS
}

pub const fn visible_category_count() -> usize {
    style::VISIBLE_CATEGORIES
}

fn is_category_region(id: &str) -> bool {
    id == "category-list" || id.starts_with("cat:")
}

fn is_row_region(id: &str) -> bool {
    id == "list"
        || id == "scroll-thumb"
        || id == "scroll-track"
        || id.starts_with("row:")
        || id.starts_with("action:")
}

fn is_action_region(id: &str) -> bool {
    id.starts_with("action:")
}

fn row_scrollbar_track_span() -> f32 {
    style::ROW_HEIGHT * visible_row_count() as f32 - ROW_SCROLLBAR_PADDING * 2.0
}

fn row_scrollbar_thumb_center_y(model: &FavoriteFriendsPanelModel) -> f32 {
    let track_top = style::LIST_Y + ROW_SCROLLBAR_PADDING;
    let track_span = row_scrollbar_track_span();
    let max = model.max_row_scroll_offset();
    if max <= 0.0 {
        return track_top + track_span * 0.5;
    }
    let visible = visible_row_count() as f32;
    let total = model.rows.len().max(visible_row_count()) as f32;
    let thumb_height =
        (track_span * visible / total).clamp(ROW_SCROLLBAR_MIN_THUMB_HEIGHT, track_span);
    let travel = (track_span - thumb_height).max(0.0);
    track_top + travel * (model.row_scroll_offset / max).clamp(0.0, 1.0) + thumb_height * 0.5
}
