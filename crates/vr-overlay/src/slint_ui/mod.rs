use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Mutex, OnceLock},
};

use slint::{
    platform::{
        self,
        software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType},
        Platform, PlatformError, PointerEventButton, WindowAdapter, WindowEvent,
    },
    ComponentHandle, Image, LogicalPosition, ModelRc, PhysicalSize, Rgba8Pixel, SharedPixelBuffer,
    SharedString, VecModel,
};

use crate::{
    AvatarBitmap, Color, DeviceChip, DeviceRole, DeviceStatus, FavoriteFriendsPanelModel, FeedKind,
    FeedLine, FeedRelation, FeedSeverity, FriendPanelCategory, FriendPanelRow,
    FriendPanelRowPrimaryAction, FriendPanelStatusTone, MainSurfaceModel, OverlaySize, RgbaFrame,
    ToastCard, WristSurfaceModel,
};

slint::include_modules!();

const DEFAULT_WIDTH: u32 = 1080;
const DEFAULT_HEIGHT: u32 = 720;

thread_local! {
    static LAST_CREATED_WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

thread_local! {
    static PLATFORM_SET: Cell<bool> = const { Cell::new(false) };
}

static PLATFORM_INIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct OverlaySlintPlatform;

impl Platform for OverlaySlintPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        LAST_CREATED_WINDOW.with(|slot| {
            *slot.borrow_mut() = Some(Rc::clone(&window));
        });
        Ok(window)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlintPanelPointerEvent {
    Moved {
        x: f32,
        y: f32,
    },
    Pressed {
        x: f32,
        y: f32,
    },
    Released {
        x: f32,
        y: f32,
    },
    Scrolled {
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    },
    Exited,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlintPanelRenderStats {
    pub elapsed: Duration,
    pub dirty_area: u64,
    pub dirty_rects: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlintPanelFrame {
    pub frame: RgbaFrame,
    pub stats: SlintPanelRenderStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlintPanelEvent {
    CategorySelected(String),
    RowClicked(String),
    ActionClicked { user_id: String, kind: String },
    ActionHoverLost { user_id: String, kind: String },
}

type AvatarImageCache = HashMap<usize, (Arc<[u8]>, Image)>;

fn avatar_cache_key(avatar: &AvatarBitmap) -> usize {
    Arc::as_ptr(&avatar.rgba) as *const u8 as usize
}

fn cached_avatar_image(
    cache: &mut AvatarImageCache,
    avatar: Option<&AvatarBitmap>,
) -> (bool, Image) {
    let Some(avatar) = avatar else {
        return (false, Image::default());
    };
    let key = avatar_cache_key(avatar);
    if let Some((held, image)) = cache.get(&key) {
        if Arc::ptr_eq(held, &avatar.rgba) {
            return (true, image.clone());
        }
    }
    let (has_avatar, image) = avatar_image(Some(avatar));
    if has_avatar {
        cache.insert(key, (Arc::clone(&avatar.rgba), image.clone()));
    }
    (has_avatar, image)
}

fn retain_avatar_images<'a>(
    cache: &mut AvatarImageCache,
    live: impl Iterator<Item = Option<&'a AvatarBitmap>>,
) {
    let live: HashSet<usize> = live.flatten().map(avatar_cache_key).collect();
    cache.retain(|key, _| live.contains(key));
}

pub struct SlintPanelHost {
    size: OverlaySize,
    window: Rc<MinimalSoftwareWindow>,
    component: FriendsPanel,
    buffer: Vec<PremultipliedRgbaColor>,
    events: Rc<RefCell<Vec<SlintPanelEvent>>>,
    last_model: Option<FavoriteFriendsPanelModel>,
    avatar_images: AvatarImageCache,
}

impl SlintPanelHost {
    pub fn new(size: OverlaySize) -> Result<Self, String> {
        let (component, window) = create_component_window(FriendsPanel::new)?;
        window.set_size(PhysicalSize::new(size.width, size.height));
        let events = Rc::new(RefCell::new(Vec::new()));
        let category_events = Rc::clone(&events);
        component.on_category_selected(move |key| {
            category_events
                .borrow_mut()
                .push(SlintPanelEvent::CategorySelected(key.to_string()));
        });
        let row_events = Rc::clone(&events);
        component.on_row_clicked(move |user_id| {
            row_events
                .borrow_mut()
                .push(SlintPanelEvent::RowClicked(user_id.to_string()));
        });
        let action_events = Rc::clone(&events);
        component.on_action_clicked(move |user_id, kind| {
            action_events
                .borrow_mut()
                .push(SlintPanelEvent::ActionClicked {
                    user_id: user_id.to_string(),
                    kind: kind.to_string(),
                });
        });
        let hover_lost_events = Rc::clone(&events);
        component.on_action_hover_lost(move |user_id, kind| {
            hover_lost_events
                .borrow_mut()
                .push(SlintPanelEvent::ActionHoverLost {
                    user_id: user_id.to_string(),
                    kind: kind.to_string(),
                });
        });
        component.show().map_err(|error| error.to_string())?;
        let buffer = vec![PremultipliedRgbaColor::default(); pixel_count(size)?];
        Ok(Self {
            size,
            window,
            component,
            buffer,
            events,
            last_model: None,
            avatar_images: AvatarImageCache::new(),
        })
    }

    pub fn size(&self) -> OverlaySize {
        self.size
    }

    pub fn set_model(&mut self, model: &FavoriteFriendsPanelModel) {
        if self.last_model.as_ref() == Some(model) {
            return;
        }
        retain_avatar_images(
            &mut self.avatar_images,
            model.rows.iter().map(|row| row.avatar.as_ref()),
        );
        self.component
            .set_panel_title(SharedString::from(model.strings.title.as_str()));
        self.component.set_status_message(SharedString::from(
            model.status_message.as_deref().unwrap_or(""),
        ));
        self.component
            .set_empty_label(SharedString::from(model.strings.empty_label.as_str()));
        self.component.set_categories(friend_category_model(model));
        self.component
            .set_rows(friend_row_model(model, &mut self.avatar_images));
        self.last_model = Some(model.clone());
    }

    pub fn dispatch(&mut self, event: SlintPanelPointerEvent) -> Result<(), String> {
        self.component
            .window()
            .try_dispatch_event(to_window_event(event))
            .map_err(|error| error.to_string())
    }

    pub fn drain_events(&mut self) -> Vec<SlintPanelEvent> {
        self.events.borrow_mut().drain(..).collect()
    }

    pub fn render_if_needed(&mut self) -> Result<Option<SlintPanelFrame>, String> {
        platform::update_timers_and_animations();
        let mut dirty_area = 0_u64;
        let mut dirty_rects = 0_usize;
        let start = Instant::now();
        let redrawn = self.window.draw_if_needed(|renderer| {
            let region = renderer.render(&mut self.buffer, self.size.width as usize);
            for (_, rect_size) in region.iter() {
                dirty_area += u64::from(rect_size.width) * u64::from(rect_size.height);
                dirty_rects += 1;
            }
        });
        let elapsed = start.elapsed();
        if !redrawn {
            return Ok(None);
        }
        Ok(Some(SlintPanelFrame {
            frame: RgbaFrame::new(self.size, pixels_to_rgba(&self.buffer)),
            stats: SlintPanelRenderStats {
                elapsed,
                dirty_area,
                dirty_rects,
            },
        }))
    }

    pub fn has_active_animations(&self) -> bool {
        self.component.window().has_active_animations()
    }
}

pub struct SlintWristRenderer {
    host: Option<SlintWristHost>,
    last_model: Option<WristSurfaceModel>,
    last_frame: Option<RgbaFrame>,
    render_count: usize,
}

impl SlintWristRenderer {
    pub fn new() -> Self {
        Self {
            host: None,
            last_model: None,
            last_frame: None,
            render_count: 0,
        }
    }

    pub fn render(&mut self, model: &WristSurfaceModel) -> Result<RgbaFrame, String> {
        if self.last_model.as_ref() == Some(model) {
            if let Some(frame) = self.last_frame.as_ref() {
                return Ok(frame.clone());
            }
        }
        let host = self.host_for_size(model.size)?;
        host.set_model(model);
        let Some(frame) = host.render_if_needed() else {
            return self.last_frame.clone().ok_or_else(|| {
                "Slint wrist renderer did not produce an initial frame".to_string()
            });
        };
        self.render_count += 1;
        self.last_model = Some(model.clone());
        self.last_frame = Some(frame.clone());
        Ok(frame)
    }

    #[cfg(test)]
    fn render_count(&self) -> usize {
        self.render_count
    }

    fn host_for_size(&mut self, size: OverlaySize) -> Result<&mut SlintWristHost, String> {
        let needs_new = self
            .host
            .as_ref()
            .map(|host| host.size() != size)
            .unwrap_or(true);
        if needs_new {
            self.host = Some(SlintWristHost::new(size)?);
            self.last_model = None;
            self.last_frame = None;
        }
        self.host
            .as_mut()
            .ok_or_else(|| "Slint wrist host is unavailable".to_string())
    }
}

impl Default for SlintWristRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SlintHmdRenderer {
    host: Option<SlintHmdHost>,
    last_model: Option<MainSurfaceModel>,
    last_frame: Option<RgbaFrame>,
    render_count: usize,
}

impl SlintHmdRenderer {
    pub fn new() -> Self {
        Self {
            host: None,
            last_model: None,
            last_frame: None,
            render_count: 0,
        }
    }

    pub fn render(&mut self, model: &MainSurfaceModel) -> Result<RgbaFrame, String> {
        if self.last_model.as_ref() == Some(model) {
            if let Some(frame) = self.last_frame.as_ref() {
                return Ok(frame.clone());
            }
        }
        let host = self.host_for_size(model.size)?;
        host.set_model(model);
        let Some(frame) = host.render_if_needed() else {
            return self
                .last_frame
                .clone()
                .ok_or_else(|| "Slint HMD renderer did not produce an initial frame".to_string());
        };
        self.render_count += 1;
        self.last_model = Some(model.clone());
        self.last_frame = Some(frame.clone());
        Ok(frame)
    }

    #[cfg(test)]
    fn render_count(&self) -> usize {
        self.render_count
    }

    fn host_for_size(&mut self, size: OverlaySize) -> Result<&mut SlintHmdHost, String> {
        let needs_new = self
            .host
            .as_ref()
            .map(|host| host.size() != size)
            .unwrap_or(true);
        if needs_new {
            self.host = Some(SlintHmdHost::new(size)?);
            self.last_model = None;
            self.last_frame = None;
        }
        self.host
            .as_mut()
            .ok_or_else(|| "Slint HMD host is unavailable".to_string())
    }
}

impl Default for SlintHmdRenderer {
    fn default() -> Self {
        Self::new()
    }
}

struct SlintWristHost {
    size: OverlaySize,
    window: Rc<MinimalSoftwareWindow>,
    component: WristPanel,
    buffer: Vec<PremultipliedRgbaColor>,
}

impl SlintWristHost {
    fn new(size: OverlaySize) -> Result<Self, String> {
        let (component, window) = create_component_window(WristPanel::new)?;
        window.set_size(PhysicalSize::new(size.width, size.height));
        component.show().map_err(|error| error.to_string())?;
        Ok(Self {
            size,
            window,
            component,
            buffer: vec![PremultipliedRgbaColor::default(); pixel_count(size)?],
        })
    }

    fn size(&self) -> OverlaySize {
        self.size
    }

    fn set_model(&mut self, model: &WristSurfaceModel) {
        self.component.set_dark_background(model.dark_background);
        self.component
            .set_accent_color(to_slint_color(model.accent));
        self.component.set_devices(wrist_device_model(model));
        self.component
            .set_feed_lines(wrist_feed_model(&model.feed_rows));
        self.component
            .set_footer_left(SharedString::from(model.footer.left.as_str()));
        self.component
            .set_footer_center(SharedString::from(model.footer.center.as_str()));
        self.component
            .set_footer_right(SharedString::from(model.footer.right.as_str()));
    }

    fn render_if_needed(&mut self) -> Option<RgbaFrame> {
        render_window_if_needed(&self.window, &mut self.buffer, self.size)
    }
}

struct SlintHmdHost {
    size: OverlaySize,
    window: Rc<MinimalSoftwareWindow>,
    component: HmdToastPanel,
    buffer: Vec<PremultipliedRgbaColor>,
    avatar_images: AvatarImageCache,
}

impl SlintHmdHost {
    fn new(size: OverlaySize) -> Result<Self, String> {
        let (component, window) = create_component_window(HmdToastPanel::new)?;
        window.set_size(PhysicalSize::new(size.width, size.height));
        component.show().map_err(|error| error.to_string())?;
        Ok(Self {
            size,
            window,
            component,
            buffer: vec![PremultipliedRgbaColor::default(); pixel_count(size)?],
            avatar_images: AvatarImageCache::new(),
        })
    }

    fn size(&self) -> OverlaySize {
        self.size
    }

    fn set_model(&mut self, model: &MainSurfaceModel) {
        retain_avatar_images(
            &mut self.avatar_images,
            model.toasts.iter().map(|toast| toast.avatar.as_ref()),
        );
        self.component.set_dark_background(model.dark_background);
        self.component
            .set_toasts(hmd_toast_model(model, &mut self.avatar_images));
    }

    fn render_if_needed(&mut self) -> Option<RgbaFrame> {
        render_window_if_needed(&self.window, &mut self.buffer, self.size)
    }
}

fn render_window_if_needed(
    window: &MinimalSoftwareWindow,
    buffer: &mut [PremultipliedRgbaColor],
    size: OverlaySize,
) -> Option<RgbaFrame> {
    platform::update_timers_and_animations();
    let redrawn = window.draw_if_needed(|renderer| {
        renderer.render(buffer, size.width as usize);
    });
    redrawn.then(|| RgbaFrame::new(size, pixels_to_rgba(buffer)))
}

fn ensure_platform() -> Result<(), String> {
    PLATFORM_SET.with(|set| {
        if set.get() {
            return Ok(());
        }
        let _guard = PLATFORM_INIT_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|error| error.to_string())?;
        if set.get() {
            return Ok(());
        }
        let result = platform::set_platform(Box::new(OverlaySlintPlatform))
            .map_err(|error| error.to_string());
        if result.is_ok() {
            set.set(true);
        }
        result
    })
}

fn create_component_window<C>(
    create: impl FnOnce() -> Result<C, PlatformError>,
) -> Result<(C, Rc<MinimalSoftwareWindow>), String>
where
    C: ComponentHandle,
{
    ensure_platform()?;
    take_last_created_window();
    let component = create().map_err(|error| error.to_string())?;
    let window = take_last_created_window()
        .ok_or_else(|| "Slint platform did not create a software window".to_string())?;
    Ok((component, window))
}

fn take_last_created_window() -> Option<Rc<MinimalSoftwareWindow>> {
    LAST_CREATED_WINDOW.with(|slot| slot.borrow_mut().take())
}

fn to_window_event(event: SlintPanelPointerEvent) -> WindowEvent {
    match event {
        SlintPanelPointerEvent::Moved { x, y } => WindowEvent::PointerMoved {
            position: LogicalPosition::new(x, y),
        },
        SlintPanelPointerEvent::Pressed { x, y } => WindowEvent::PointerPressed {
            position: LogicalPosition::new(x, y),
            button: PointerEventButton::Left,
        },
        SlintPanelPointerEvent::Released { x, y } => WindowEvent::PointerReleased {
            position: LogicalPosition::new(x, y),
            button: PointerEventButton::Left,
        },
        SlintPanelPointerEvent::Scrolled {
            x,
            y,
            delta_x,
            delta_y,
        } => WindowEvent::PointerScrolled {
            position: LogicalPosition::new(x, y),
            delta_x,
            delta_y,
        },
        SlintPanelPointerEvent::Exited => WindowEvent::PointerExited,
    }
}

fn pixel_count(size: OverlaySize) -> Result<usize, String> {
    RgbaFrame::expected_byte_len(size)
        .map(|bytes| bytes / 4)
        .ok_or_else(|| format!("invalid Slint panel size {}x{}", size.width, size.height))
}

fn pixels_to_rgba(pixels: &[PremultipliedRgbaColor]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for pixel in pixels {
        rgba.push(pixel.red);
        rgba.push(pixel.green);
        rgba.push(pixel.blue);
        rgba.push(pixel.alpha);
    }
    rgba
}

fn to_slint_color(color: crate::Color) -> slint::Color {
    slint::Color::from_argb_u8(color.a, color.r, color.g, color.b)
}

fn hmd_toast_model(
    model: &MainSurfaceModel,
    cache: &mut AvatarImageCache,
) -> ModelRc<HmdToastItem> {
    ModelRc::new(VecModel::from(
        model
            .toasts
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(|toast| hmd_toast_item(toast, model.accent, cache))
            .collect::<Vec<_>>(),
    ))
}

fn hmd_toast_item(
    toast: &ToastCard,
    accent: crate::Color,
    cache: &mut AvatarImageCache,
) -> HmdToastItem {
    let (has_avatar, avatar) = cached_avatar_image(cache, toast.avatar.as_ref());
    HmdToastItem {
        actor: SharedString::from(hmd_actor_text(toast)),
        action: SharedString::from(toast.action.as_str()),
        context: SharedString::from(toast.context.as_deref().unwrap_or_default()),
        avatar,
        has_avatar,
        relation_color: hmd_relation_color(toast.relation),
        severity_color: hmd_severity_color(toast.severity, accent),
    }
}

fn hmd_actor_text(toast: &ToastCard) -> String {
    let name = toast.actor_name.trim();
    if toast.relation == FeedRelation::Favorite && !name.is_empty() {
        format!("{name} ★")
    } else {
        name.to_string()
    }
}

fn avatar_image(avatar: Option<&AvatarBitmap>) -> (bool, Image) {
    let Some(avatar) = avatar else {
        return (false, Image::default());
    };
    let expected_len = avatar
        .width
        .checked_mul(avatar.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .map(|bytes| bytes as usize);
    if expected_len != Some(avatar.rgba.len()) {
        return (false, Image::default());
    }
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        &avatar.rgba,
        avatar.width,
        avatar.height,
    );
    (true, Image::from_rgba8(buffer))
}

fn friend_category_model(model: &FavoriteFriendsPanelModel) -> ModelRc<FriendPanelCategoryItem> {
    ModelRc::new(VecModel::from(
        model
            .categories
            .iter()
            .map(|category| friend_category_item(category, &model.selected_category_key))
            .collect::<Vec<_>>(),
    ))
}

fn friend_category_item(
    category: &FriendPanelCategory,
    selected_key: &str,
) -> FriendPanelCategoryItem {
    FriendPanelCategoryItem {
        key: SharedString::from(category.key.as_str()),
        label: SharedString::from(category.label.as_str()),
        count: SharedString::from(category.count.to_string().as_str()),
        selected: category.key == selected_key,
    }
}

fn friend_row_model(
    model: &FavoriteFriendsPanelModel,
    cache: &mut AvatarImageCache,
) -> ModelRc<FriendPanelRowItem> {
    ModelRc::new(VecModel::from(
        model
            .rows
            .iter()
            .map(|row| friend_row_item(row, model, cache))
            .collect::<Vec<_>>(),
    ))
}

fn friend_row_item(
    row: &FriendPanelRow,
    model: &FavoriteFriendsPanelModel,
    cache: &mut AvatarImageCache,
) -> FriendPanelRowItem {
    let (has_avatar, avatar) = cached_avatar_image(cache, row.avatar.as_ref());
    let (primary_label, primary_kind, has_primary) = match row.actions.primary {
        Some(FriendPanelRowPrimaryAction::Open) => {
            (model.strings.open_label.as_str(), "open", true)
        }
        Some(FriendPanelRowPrimaryAction::Request) => {
            (model.strings.request_label.as_str(), "request", true)
        }
        None => ("", "", false),
    };
    let armed_primary = has_primary
        && model.armed_action_region_id.as_deref()
            == Some(friend_action_id(&row.user_id, primary_kind).as_str());
    let armed_invite = row.actions.invite
        && model.armed_action_region_id.as_deref()
            == Some(friend_action_id(&row.user_id, "invite").as_str());
    let note_text = labeled_optional_text(&model.strings.note_label, row.note.as_deref());
    let memo_text = labeled_optional_text(&model.strings.memo_label, row.memo.as_deref());
    let section_label = row.section_label.as_deref().unwrap_or_default();
    FriendPanelRowItem {
        section_label: SharedString::from(section_label),
        user_id: SharedString::from(row.user_id.as_str()),
        display_name: SharedString::from(row.display_name.as_str()),
        location_text: SharedString::from(row.location_text.as_str()),
        traveling_text: SharedString::from(
            row.traveling_text
                .as_deref()
                .unwrap_or(row.location_text.as_str()),
        ),
        note_text: SharedString::from(note_text.as_str()),
        memo_text: SharedString::from(memo_text.as_str()),
        avatar,
        has_avatar,
        status_color: to_slint_color(friend_status_color(row.status)),
        name_color: to_slint_color(friend_name_color(row.status)),
        primary_label: SharedString::from(primary_label),
        primary_kind: SharedString::from(primary_kind),
        has_primary,
        has_invite: row.actions.invite,
        invite_label: SharedString::from(model.strings.invite_label.as_str()),
        armed_primary,
        armed_invite,
        is_traveling: row.is_traveling,
        is_section: row.section_label.is_some(),
    }
}

fn labeled_optional_text(label: &str, value: Option<&str>) -> String {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return String::new();
    };
    format!("{label}: {value}")
}

fn friend_action_id(user_id: &str, kind: &str) -> String {
    format!("action:{user_id}:{kind}")
}

fn friend_status_color(status: FriendPanelStatusTone) -> Color {
    match status {
        FriendPanelStatusTone::Online => Color::rgba(34, 197, 94, 255),
        FriendPanelStatusTone::Active => Color::rgba(45, 212, 191, 255),
        FriendPanelStatusTone::Busy => Color::rgba(248, 113, 113, 255),
        FriendPanelStatusTone::AskMe => Color::rgba(251, 191, 36, 255),
        FriendPanelStatusTone::Offline => Color::rgba(100, 116, 139, 255),
    }
}

fn friend_name_color(status: FriendPanelStatusTone) -> Color {
    match status {
        FriendPanelStatusTone::Offline => Color::rgba(148, 163, 184, 255),
        _ => Color::rgba(250, 204, 21, 255),
    }
}

fn hmd_relation_color(relation: FeedRelation) -> slint::Color {
    match relation {
        FeedRelation::Favorite => slint::Color::from_rgb_u8(245, 205, 84),
        FeedRelation::Friend => slint::Color::from_rgb_u8(246, 246, 246),
        FeedRelation::None => slint::Color::from_rgb_u8(238, 238, 238),
    }
}

fn hmd_severity_color(severity: FeedSeverity, accent: crate::Color) -> slint::Color {
    match severity {
        FeedSeverity::Important => slint::Color::from_rgb_u8(245, 158, 11),
        FeedSeverity::Warning => slint::Color::from_rgb_u8(239, 68, 68),
        FeedSeverity::Normal => to_slint_color(accent),
    }
}

const WRIST_TEXT: Color = Color::rgba(238, 238, 238, 255);
const WRIST_FRIEND_TEXT: Color = Color::rgba(246, 246, 246, 255);
const WRIST_FAVORITE_TEXT: Color = Color::rgba(245, 205, 84, 255);
const WRIST_MUTED_TEXT: Color = Color::rgba(168, 168, 168, 255);
const WRIST_LOW: Color = Color::rgba(245, 158, 11, 255);
const WRIST_CRITICAL: Color = Color::rgba(239, 68, 68, 255);
const WRIST_NORMAL: Color = Color::rgba(34, 197, 94, 255);
const WRIST_WARNING: Color = Color::rgba(251, 191, 36, 255);

#[derive(Clone, Debug)]
struct WristDeviceToken {
    label: String,
    status: DeviceStatus,
    battery_percent: Option<u8>,
    aggregate_count: Option<usize>,
    abnormal: bool,
    draw_battery: bool,
}

impl WristDeviceToken {
    fn specific(device: &DeviceChip, label: String) -> Self {
        Self {
            label,
            status: device.status,
            battery_percent: device.battery_percent,
            aggregate_count: None,
            abnormal: is_abnormal_device_status(device.status),
            draw_battery: true,
        }
    }

    fn aggregate(label: String, count: usize, abnormal: bool) -> Self {
        Self {
            label,
            status: if abnormal {
                DeviceStatus::TrackingWarning
            } else {
                DeviceStatus::Normal
            },
            battery_percent: None,
            aggregate_count: Some(count),
            abnormal,
            draw_battery: false,
        }
    }

    fn percent_text(&self, show_percent: bool) -> Option<String> {
        if self.aggregate_count.is_some() || !show_percent {
            return None;
        }
        self.battery_percent.map(|percent| format!("{percent}%"))
    }
}

fn wrist_device_model(model: &WristSurfaceModel) -> ModelRc<WristDeviceItem> {
    ModelRc::new(VecModel::from(
        wrist_device_tokens(&model.devices, model.size.width as f32)
            .into_iter()
            .map(|token| wrist_device_item(&token, model.show_battery_percent))
            .collect::<Vec<_>>(),
    ))
}

fn wrist_device_tokens(devices: &[DeviceChip], width: f32) -> Vec<WristDeviceToken> {
    let mut tokens = Vec::new();
    push_wrist_role_token(&mut tokens, devices, DeviceRole::Hmd, "HMD");
    push_wrist_role_token(&mut tokens, devices, DeviceRole::LeftController, "L");
    push_wrist_role_token(&mut tokens, devices, DeviceRole::RightController, "R");

    let abnormal_tracker_limit = abnormal_tracker_display_limit(width);
    let mut abnormal_trackers = devices
        .iter()
        .filter(|device| {
            device.role == DeviceRole::Tracker && is_abnormal_device_status(device.status)
        })
        .collect::<Vec<_>>();
    abnormal_trackers.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| tracker_index(&left.label).cmp(&tracker_index(&right.label)))
    });
    for device in abnormal_trackers.iter().take(abnormal_tracker_limit) {
        tokens.push(WristDeviceToken::specific(device, device.label.clone()));
    }
    if abnormal_trackers.len() > abnormal_tracker_limit {
        tokens.push(WristDeviceToken::aggregate(
            format!("+{}", abnormal_trackers.len() - abnormal_tracker_limit),
            abnormal_trackers.len() - abnormal_tracker_limit,
            true,
        ));
    }

    let normal_tracker_count = devices
        .iter()
        .filter(|device| {
            device.role == DeviceRole::Tracker && !is_abnormal_device_status(device.status)
        })
        .count();
    if normal_tracker_count > 0 {
        tokens.push(WristDeviceToken::aggregate(
            format!("T×{normal_tracker_count}"),
            normal_tracker_count,
            false,
        ));
    }

    for device in devices
        .iter()
        .filter(|device| {
            device.role == DeviceRole::Other && is_abnormal_device_status(device.status)
        })
        .take(2)
    {
        tokens.push(WristDeviceToken::specific(device, device.label.clone()));
    }
    tokens
}

fn push_wrist_role_token(
    tokens: &mut Vec<WristDeviceToken>,
    devices: &[DeviceChip],
    role: DeviceRole,
    label: &str,
) {
    if let Some(device) = devices.iter().find(|device| device.role == role) {
        tokens.push(WristDeviceToken::specific(device, label.to_string()));
    }
}

fn wrist_device_item(token: &WristDeviceToken, show_percent: bool) -> WristDeviceItem {
    let percent = token.percent_text(show_percent).unwrap_or_default();
    let label_color = if token.aggregate_count.is_some() && token.abnormal {
        wrist_status_color(token.status)
    } else {
        WRIST_MUTED_TEXT
    };
    let percent_color = if is_abnormal_device_status(token.status) {
        wrist_status_color(token.status)
    } else {
        WRIST_MUTED_TEXT
    };
    WristDeviceItem {
        label: SharedString::from(token.label.as_str()),
        percent: SharedString::from(percent.as_str()),
        label_color: to_slint_color(label_color),
        percent_color: to_slint_color(percent_color),
        battery_color: to_slint_color(wrist_status_color(token.status)),
        battery_fill: battery_fill_ratio(token.status, token.battery_percent),
        show_percent: !percent.is_empty(),
        show_battery: token.draw_battery,
    }
}

fn abnormal_tracker_display_limit(width: f32) -> usize {
    if width >= 600.0 {
        4
    } else if width >= 540.0 {
        3
    } else {
        2
    }
}

fn tracker_index(label: &str) -> u32 {
    label
        .trim()
        .strip_prefix('T')
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(u32::MAX)
}

fn battery_fill_ratio(status: DeviceStatus, battery_percent: Option<u8>) -> f32 {
    if let Some(percent) = battery_percent {
        return (percent as f32 / 100.0).clamp(0.0, 1.0);
    }
    match status {
        DeviceStatus::Normal | DeviceStatus::Charging => 1.0,
        DeviceStatus::LowBattery => 0.3,
        DeviceStatus::CriticalBattery => 0.15,
        DeviceStatus::TrackingWarning => 0.5,
        DeviceStatus::Disconnected => 0.0,
    }
}

fn is_abnormal_device_status(status: DeviceStatus) -> bool {
    matches!(
        status,
        DeviceStatus::LowBattery
            | DeviceStatus::CriticalBattery
            | DeviceStatus::TrackingWarning
            | DeviceStatus::Disconnected
    )
}

fn wrist_status_color(status: DeviceStatus) -> Color {
    match status {
        DeviceStatus::Normal | DeviceStatus::Charging => WRIST_NORMAL,
        DeviceStatus::LowBattery => WRIST_LOW,
        DeviceStatus::CriticalBattery | DeviceStatus::Disconnected => WRIST_CRITICAL,
        DeviceStatus::TrackingWarning => WRIST_WARNING,
    }
}

fn wrist_feed_model(rows: &[FeedLine]) -> ModelRc<WristFeedItem> {
    ModelRc::new(VecModel::from(
        rows.iter().map(wrist_feed_item).collect::<Vec<_>>(),
    ))
}

fn wrist_feed_item(row: &FeedLine) -> WristFeedItem {
    let actor = row.actor_text.trim();
    let (actor, detail, has_actor) = if actor.is_empty() || row.relation == FeedRelation::None {
        ("", row.detail.trim().to_string(), false)
    } else {
        (actor, detail_without_actor(row.detail.trim(), actor), true)
    };
    WristFeedItem {
        time: SharedString::from(row.time_text.trim()),
        actor: SharedString::from(actor),
        detail: SharedString::from(detail.as_str()),
        actor_color: to_slint_color(wrist_relation_color(row.relation)),
        detail_color: to_slint_color(wrist_detail_color(row)),
        severity_color: to_slint_color(wrist_severity_color(row.severity)),
        has_actor,
        show_severity: row.severity != FeedSeverity::Normal,
    }
}

fn detail_without_actor(detail: &str, actor: &str) -> String {
    detail
        .strip_prefix(actor)
        .map(str::trim_start)
        .unwrap_or(detail)
        .to_string()
}

fn wrist_relation_color(relation: FeedRelation) -> Color {
    match relation {
        FeedRelation::Favorite => WRIST_FAVORITE_TEXT,
        FeedRelation::Friend => WRIST_FRIEND_TEXT,
        FeedRelation::None => WRIST_TEXT,
    }
}

fn wrist_detail_color(row: &FeedLine) -> Color {
    match row.kind {
        FeedKind::Media => WRIST_MUTED_TEXT,
        _ => WRIST_TEXT,
    }
}

fn wrist_severity_color(severity: FeedSeverity) -> Color {
    match severity {
        FeedSeverity::Important => WRIST_LOW,
        FeedSeverity::Warning => WRIST_CRITICAL,
        FeedSeverity::Normal => WRIST_NORMAL,
    }
}

pub fn default_slint_panel_size() -> OverlaySize {
    OverlaySize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AvatarBitmap, DeviceChip, DeviceRole, DeviceStatus, FavoriteFriendsPanelModel, FeedKind,
        FeedLine, FeedRelation, FeedSeverity, FriendPanelCategory, FriendPanelRow,
        FriendPanelRowActions, FriendPanelRowPrimaryAction, FriendPanelStatusTone,
        MainSurfaceModel, OverlayFooter, ToastCard, WristSurfaceModel,
    };
    use std::{sync::Arc, thread};

    #[test]
    fn slint_platform_init_is_available_on_each_render_thread() {
        ensure_platform().unwrap();
        thread::spawn(|| {
            let mut renderer = SlintWristRenderer::new();
            let frame = renderer.render(&sample_wrist_model()).unwrap();
            assert_eq!(frame.size, OverlaySize::new(512, 512));
        })
        .join()
        .unwrap();
    }

    #[test]
    fn slint_panel_host_renders_friends_model_and_dispatches_pointer_input() {
        let mut host = SlintPanelHost::new(OverlaySize::new(1080, 720)).unwrap();
        host.set_model(&sample_friends_model());
        let initial = host.render_if_needed().unwrap().unwrap();
        assert_eq!(initial.frame.size, OverlaySize::new(1080, 720));
        assert_eq!(initial.frame.data[3], 240);
        assert!(host.render_if_needed().unwrap().is_none());
        host.dispatch(SlintPanelPointerEvent::Moved { x: 350.0, y: 190.0 })
            .unwrap();
        let hover = host.render_if_needed().unwrap().unwrap();

        assert_ne!(initial.frame.data, hover.frame.data);
        assert!(hover.stats.dirty_area > 0);
        assert!(hover.stats.dirty_area < u64::from(1080_u32 * 720_u32));
    }

    #[test]
    fn slint_wrist_renderer_reuses_cached_frame_for_equal_model() {
        let mut renderer = SlintWristRenderer::new();
        let model = sample_wrist_model();

        let first = renderer.render(&model).unwrap();
        let second = renderer.render(&model).unwrap();

        assert_eq!(first, second);
        assert_eq!(renderer.render_count(), 1);

        let mut changed = model.clone();
        changed.footer.right = "12:35".to_string();
        let third = renderer.render(&changed).unwrap();

        assert_ne!(first, third);
        assert_eq!(renderer.render_count(), 2);
    }

    #[test]
    fn slint_hmd_renderer_reuses_cached_frame_for_equal_model() {
        let mut renderer = SlintHmdRenderer::new();
        let model = sample_main_model();

        let first = renderer.render(&model).unwrap();
        let second = renderer.render(&model).unwrap();

        assert_eq!(first, second);
        assert_eq!(renderer.render_count(), 1);

        let mut changed = model.clone();
        changed.toasts[0].action = "joined a private instance".to_string();
        let third = renderer.render(&changed).unwrap();

        assert_ne!(first, third);
        assert_eq!(renderer.render_count(), 2);
    }

    #[test]
    fn slint_hmd_renderer_shows_late_arriving_avatar() {
        let mut renderer = SlintHmdRenderer::new();
        let mut model = sample_main_model();
        let avatar = model.toasts[0].avatar.take();

        let without_avatar = renderer.render(&model).unwrap();

        model.toasts[0].avatar = avatar;
        let with_avatar = renderer.render(&model).unwrap();

        assert_ne!(without_avatar, with_avatar);
        assert_eq!(renderer.render_count(), 2);
    }

    #[test]
    fn wrist_device_tokens_prioritize_abnormal_trackers_and_filter_normal_other_devices() {
        let devices = vec![
            device("HMD", DeviceRole::Hmd, DeviceStatus::Normal, Some(90), 10),
            device(
                "L",
                DeviceRole::LeftController,
                DeviceStatus::LowBattery,
                Some(20),
                30,
            ),
            device(
                "R",
                DeviceRole::RightController,
                DeviceStatus::Normal,
                Some(80),
                10,
            ),
            device(
                "T1",
                DeviceRole::Tracker,
                DeviceStatus::TrackingWarning,
                None,
                30,
            ),
            device(
                "T2",
                DeviceRole::Tracker,
                DeviceStatus::CriticalBattery,
                Some(7),
                40,
            ),
            device(
                "T3",
                DeviceRole::Tracker,
                DeviceStatus::Normal,
                Some(70),
                10,
            ),
            device(
                "T5",
                DeviceRole::Tracker,
                DeviceStatus::LowBattery,
                Some(21),
                30,
            ),
            device(
                "Dongle",
                DeviceRole::Other,
                DeviceStatus::Disconnected,
                None,
                40,
            ),
            device("Camera", DeviceRole::Other, DeviceStatus::Normal, None, 10),
        ];

        let labels = wrist_device_tokens(&devices, 512.0)
            .into_iter()
            .map(|token| token.label)
            .collect::<Vec<_>>();

        assert_eq!(labels, ["HMD", "L", "R", "T2", "T1", "+1", "T×1", "Dongle"]);
    }

    #[test]
    fn wrist_feed_item_preserves_actor_detail_and_muted_media_detail() {
        let favorite = FeedLine {
            time_text: "16:31".to_string(),
            kind: FeedKind::Invite,
            actor_text: "Ada".to_string(),
            detail: "Ada invited you".to_string(),
            relation: FeedRelation::Favorite,
            severity: FeedSeverity::Important,
        };
        let media = FeedLine {
            time_text: String::new(),
            kind: FeedKind::Media,
            actor_text: "Player".to_string(),
            detail: "Muted media row".to_string(),
            relation: FeedRelation::None,
            severity: FeedSeverity::Normal,
        };

        let favorite_item = wrist_feed_item(&favorite);
        let media_item = wrist_feed_item(&media);

        assert!(favorite_item.has_actor);
        assert_eq!(favorite_item.actor.to_string(), "Ada");
        assert_eq!(favorite_item.detail.to_string(), "invited you");
        assert!(favorite_item.show_severity);
        assert!(!media_item.has_actor);
        assert_eq!(media_item.detail.to_string(), "Muted media row");
        assert_eq!(media_item.detail_color, to_slint_color(WRIST_MUTED_TEXT));
    }

    fn sample_wrist_model() -> WristSurfaceModel {
        WristSurfaceModel {
            size: OverlaySize::new(512, 512),
            dark_background: true,
            show_battery_percent: true,
            devices: vec![
                DeviceChip {
                    label: "HMD".to_string(),
                    role: DeviceRole::Hmd,
                    status: DeviceStatus::Normal,
                    battery_percent: Some(82),
                    text: "82".to_string(),
                    priority: 10,
                },
                DeviceChip {
                    label: "L".to_string(),
                    role: DeviceRole::LeftController,
                    status: DeviceStatus::LowBattery,
                    battery_percent: Some(18),
                    text: "18 low".to_string(),
                    priority: 20,
                },
            ],
            feed_rows: vec![FeedLine {
                time_text: "16:31".to_string(),
                kind: FeedKind::Invite,
                actor_text: "Ada".to_string(),
                detail: "Ada invited you to 测试世界".to_string(),
                relation: FeedRelation::Favorite,
                severity: FeedSeverity::Important,
            }],
            footer: OverlayFooter {
                left: "8 players".to_string(),
                center: "Instance 12m".to_string(),
                right: "12:34".to_string(),
            },
            accent: crate::Color::rgba(94, 234, 212, 255),
            captured_at_ms: 1_717_200_000_000,
        }
    }

    fn sample_main_model() -> MainSurfaceModel {
        MainSurfaceModel {
            size: OverlaySize::new(960, 528),
            dark_background: true,
            accent: crate::Color::rgba(94, 234, 212, 255),
            toasts: vec![ToastCard {
                actor_name: "Ada".to_string(),
                relation: FeedRelation::Favorite,
                action: "joined your instance".to_string(),
                context: Some("测试世界".to_string()),
                severity: FeedSeverity::Important,
                avatar: Some(AvatarBitmap {
                    width: 2,
                    height: 2,
                    rgba: Arc::from(vec![
                        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
                    ]),
                }),
            }],
        }
    }

    fn sample_friends_model() -> FavoriteFriendsPanelModel {
        FavoriteFriendsPanelModel {
            categories: vec![FriendPanelCategory {
                key: "all".to_string(),
                label: "All".to_string(),
                count: 1,
            }],
            rows: vec![FriendPanelRow {
                section_label: None,
                user_id: "usr_friend".to_string(),
                display_name: "Ada".to_string(),
                status: FriendPanelStatusTone::Online,
                location_text: "测试世界 Public".to_string(),
                is_traveling: false,
                traveling_text: None,
                note: Some("VRChat note".to_string()),
                memo: Some("Local memo".to_string()),
                avatar: Some(AvatarBitmap {
                    width: 2,
                    height: 2,
                    rgba: Arc::from(vec![
                        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
                    ]),
                }),
                actions: FriendPanelRowActions {
                    primary: Some(FriendPanelRowPrimaryAction::Open),
                    invite: true,
                },
            }],
            ..FavoriteFriendsPanelModel::default()
        }
    }

    fn device(
        label: &str,
        role: DeviceRole,
        status: DeviceStatus,
        battery_percent: Option<u8>,
        priority: u8,
    ) -> DeviceChip {
        DeviceChip {
            label: label.to_string(),
            role,
            status,
            battery_percent,
            text: String::new(),
            priority,
        }
    }
}
