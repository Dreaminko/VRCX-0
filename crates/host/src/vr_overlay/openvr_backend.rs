use std::collections::HashMap;
use std::time::{Duration, Instant};

use openvr::{
    button_id,
    overlay::OverlayHandle,
    pose::Matrix3x4,
    property::{
        ControllerRoleHint_Int32, DeviceBatteryPercentage_Float, DeviceIsCharging_Bool,
        DeviceProvidesBatteryStatus_Bool, ModelNumber_String, SerialNumber_String,
        TrackingSystemName_String,
    },
    system::event::Event,
    tracked_device_index, ApplicationType, Context, Overlay, System, TrackedControllerRole,
    TrackedDeviceClass, TrackedDeviceIndex, TrackingUniverseOrigin, MAX_TRACKED_DEVICE_COUNT,
};
use vrcx_0_vr_overlay::{OverlaySurfaceId, RgbaFrame, MAIN_SURFACE_ID};

use super::{
    actor::{OverlayBackend, TickOutcome},
    policy::WristVisibilityPolicy,
    types::{
        BackendStartError, OverlayActivationButton, OverlayPlacement, OverlaySurfaceConfig,
        VrDeviceSnapshot, VrDeviceStatus,
    },
};

const WRIST_VISIBLE_FRAME_UPLOAD_INTERVAL: Duration = Duration::from_secs(2);
const MAIN_VISIBLE_FRAME_UPLOAD_INTERVAL: Duration = Duration::from_millis(100);
const SURFACE_FADE_DURATION: Duration = Duration::from_millis(240);

pub struct OpenVrOverlayBackend {
    context: Option<Context>,
    overlay: Option<Overlay>,
    system: Option<System>,
    surfaces: HashMap<OverlaySurfaceId, OpenVrSurface>,
}

struct OpenVrSurface {
    handle: OverlayHandle,
    config: OverlaySurfaceConfig,
    transform_device: Option<TrackedDeviceIndex>,
    policy: WristVisibilityPolicy,
    visible: bool,
    active: bool,
    pending_frame: Option<RgbaFrame>,
    last_visible_frame_upload_at: Option<Instant>,
    current_alpha: f32,
    target_alpha: f32,
    fade: Option<SurfaceFade>,
    hide_after_fade: bool,
}

#[derive(Clone, Copy)]
struct SurfaceFade {
    from: f32,
    to: f32,
    started_at: Instant,
}

#[derive(Clone)]
struct SurfaceUpdateCandidate {
    surface_id: OverlaySurfaceId,
    handle: OverlayHandle,
    config: OverlaySurfaceConfig,
    transform_device: Option<TrackedDeviceIndex>,
    policy: WristVisibilityPolicy,
}

impl OpenVrOverlayBackend {
    pub fn new() -> Self {
        Self {
            context: None,
            overlay: None,
            system: None,
            surfaces: HashMap::new(),
        }
    }
}

impl Default for OpenVrOverlayBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayBackend for OpenVrOverlayBackend {
    fn start(&mut self) -> Result<(), BackendStartError> {
        if self.context.is_some() && self.overlay.is_some() && self.system.is_some() {
            return Ok(());
        }

        let context = unsafe { openvr::init(ApplicationType::Background) }
            .map_err(|error| init_start_error("OpenVR init failed", error))?;
        let overlay = context
            .overlay()
            .map_err(|error| init_start_error("OpenVR overlay interface failed", error))?;
        let system = context
            .system()
            .map_err(|error| init_start_error("OpenVR system interface failed", error))?;
        self.context = Some(context);
        self.overlay = Some(overlay);
        self.system = Some(system);
        Ok(())
    }

    fn register_surface(&mut self, config: OverlaySurfaceConfig) -> Result<(), String> {
        self.start().map_err(|error| error.message)?;
        let surface_id = config.surface_id.clone();
        if self.surfaces.contains_key(&surface_id) {
            self.apply_config(&config)?;
            if let Some(surface) = self.surfaces.get_mut(&surface_id) {
                surface.config = config;
                surface.active = true;
            }
            return Ok(());
        }

        let overlay = self
            .overlay
            .as_mut()
            .ok_or_else(|| "OpenVR overlay is not started".to_string())?;
        let handle = overlay
            .create_overlay(
                &format!("vrcx.{}\0", config.surface_id.as_str()),
                &format!("VRCX {} Overlay\0", config.surface_id.as_str()),
            )
            .map_err(|error| format!("create overlay failed: {error:?}"))?;
        self.surfaces.insert(
            surface_id,
            OpenVrSurface {
                handle,
                config: config.clone(),
                transform_device: None,
                policy: WristVisibilityPolicy::default(),
                visible: false,
                active: true,
                pending_frame: None,
                last_visible_frame_upload_at: None,
                current_alpha: 1.0,
                target_alpha: 1.0,
                fade: None,
                hide_after_fade: false,
            },
        );
        self.apply_config(&config)
    }

    fn update_frame(
        &mut self,
        surface_id: &OverlaySurfaceId,
        frame: RgbaFrame,
    ) -> Result<(), String> {
        let handle = {
            let surface = self.surfaces.get_mut(surface_id).ok_or_else(|| {
                format!(
                    "overlay surface '{}' is not registered",
                    surface_id.as_str()
                )
            })?;
            if surface.visible {
                let now = Instant::now();
                let can_upload = surface
                    .last_visible_frame_upload_at
                    .map(|last| {
                        now.saturating_duration_since(last)
                            >= visible_frame_upload_interval(surface_id)
                    })
                    .unwrap_or(true);
                if !can_upload {
                    surface.pending_frame = Some(frame);
                    return Ok(());
                }
                surface.pending_frame = None;
                surface.last_visible_frame_upload_at = Some(now);
                surface.handle
            } else {
                surface.pending_frame = Some(frame);
                return Ok(());
            }
        };

        if let Err(error) = self.upload_frame(handle, &frame) {
            if let Some(surface) = self.surfaces.get_mut(surface_id) {
                surface.pending_frame = Some(frame);
                surface.last_visible_frame_upload_at = None;
            }
            return Err(error);
        }
        Ok(())
    }

    fn show(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        if surface_fades(surface_id) {
            return self.show_with_fade(surface_id);
        }
        self.set_visibility(surface_id, true)
    }

    fn hide(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        if surface_fades(surface_id) {
            return self.hide_with_fade(surface_id);
        }
        self.set_visibility(surface_id, false)
    }

    fn set_alpha(&mut self, surface_id: &OverlaySurfaceId, alpha: f32) -> Result<(), String> {
        let alpha = alpha.clamp(0.0, 1.0);
        let apply_now = {
            let surface = self.surfaces.get_mut(surface_id).ok_or_else(|| {
                format!(
                    "overlay surface '{}' is not registered",
                    surface_id.as_str()
                )
            })?;
            surface.target_alpha = alpha;
            match surface.fade.as_mut() {
                Some(fade) if !surface.hide_after_fade => {
                    fade.to = alpha;
                    false
                }
                Some(_) => false,
                None => true,
            }
        };
        if !apply_now {
            return Ok(());
        }
        self.apply_alpha(surface_id, alpha)
    }

    fn unregister_surface(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        if !self.surfaces.contains_key(surface_id) {
            return Ok(());
        }
        self.set_visibility(surface_id, false)?;
        if let Some(surface) = self.surfaces.get_mut(surface_id) {
            surface.active = false;
            surface.policy.close();
        }
        Ok(())
    }

    fn snapshot_devices(&mut self) -> Result<Vec<VrDeviceSnapshot>, String> {
        self.start().map_err(|error| error.message)?;
        let system = self
            .system
            .as_ref()
            .ok_or_else(|| "OpenVR system interface is not started".to_string())?;
        Ok(snapshot_openvr_devices(system))
    }

    fn tick(&mut self) -> TickOutcome {
        if self.poll_runtime_quit() {
            self.clear_runtime_handles();
            return TickOutcome::RuntimeQuit;
        }
        if let Err(error) = self.update_button_visibility() {
            tracing::warn!(error = %error, "failed to update VR overlay button visibility");
        }
        if let Err(error) = self.advance_fades() {
            tracing::warn!(error = %error, "failed to advance VR overlay fade");
        }
        TickOutcome::Continue
    }

    fn stop(&mut self) {
        let surface_ids = self.surfaces.keys().cloned().collect::<Vec<_>>();
        for surface_id in surface_ids {
            let _ = self.set_visibility(&surface_id, false);
        }
        self.clear_runtime_handles();
    }
}

fn surface_fades(surface_id: &OverlaySurfaceId) -> bool {
    surface_id.as_str() == MAIN_SURFACE_ID
}

fn surface_uses_wrist_policy(config: &OverlaySurfaceConfig) -> bool {
    match &config.placement {
        OverlayPlacement::TrackedDeviceRelative { device_hint } => !device_hint.starts_with("hmd"),
    }
}

fn visible_frame_upload_interval(surface_id: &OverlaySurfaceId) -> Duration {
    if surface_id.as_str() == MAIN_SURFACE_ID {
        MAIN_VISIBLE_FRAME_UPLOAD_INTERVAL
    } else {
        WRIST_VISIBLE_FRAME_UPLOAD_INTERVAL
    }
}

impl OpenVrOverlayBackend {
    fn poll_runtime_quit(&self) -> bool {
        let Some(system) = &self.system else {
            return false;
        };
        while let Some(info) = system.poll_next_event() {
            if let Event::Quit(_) = info.event {
                system.acknowledge_quit_exiting();
                return true;
            }
        }
        false
    }

    fn clear_runtime_handles(&mut self) {
        self.surfaces.clear();
        self.overlay = None;
        self.system = None;
        self.context = None;
    }

    fn update_button_visibility(&mut self) -> Result<(), String> {
        if self.surfaces.is_empty() {
            return Ok(());
        }
        let system = self
            .system
            .as_ref()
            .ok_or_else(|| "OpenVR system interface is not started".to_string())?;
        let candidates = self
            .surfaces
            .iter()
            .filter(|(_, surface)| surface.active && surface_uses_wrist_policy(&surface.config))
            .map(|(surface_id, surface)| SurfaceUpdateCandidate {
                surface_id: surface_id.clone(),
                handle: surface.handle,
                config: surface.config.clone(),
                transform_device: surface.transform_device,
                policy: surface.policy,
            })
            .collect::<Vec<_>>();

        let overlay = self
            .overlay
            .as_mut()
            .ok_or_else(|| "OpenVR overlay is not started".to_string())?;
        let now = Instant::now();
        let mut surface_updates = Vec::new();
        let mut visibility_updates = Vec::new();
        for candidate in candidates {
            let mut transform_device = candidate.transform_device;
            let mut policy = candidate.policy;

            if let Ok(device) = resolve_device(system, &candidate.config.placement) {
                if transform_device != Some(device) {
                    overlay
                        .set_transform_tracked_device_relative(
                            candidate.handle,
                            device,
                            &surface_transform(&candidate.config.placement),
                        )
                        .map_err(|error| format!("set overlay transform failed: {error:?}"))?;
                    tracing::debug!(
                        surface_id = candidate.surface_id.as_str(),
                        device_index = device.0,
                        placement = ?candidate.config.placement,
                        "resolved VR overlay tracked device"
                    );
                }
                transform_device = Some(device);
                if device_button_pressed(system, device, candidate.config.activation_button) {
                    policy.open(now);
                }
            }

            let visible = policy.evaluate(now, transform_device.is_some());
            surface_updates.push((candidate.surface_id.clone(), transform_device, policy));
            visibility_updates.push((candidate.surface_id, visible));
        }

        for (surface_id, transform_device, policy) in surface_updates {
            if let Some(surface) = self.surfaces.get_mut(&surface_id) {
                surface.transform_device = transform_device;
                surface.policy = policy;
            }
        }
        for (surface_id, visible) in visibility_updates {
            self.set_visibility(&surface_id, visible)?;
        }
        Ok(())
    }

    fn apply_config(&mut self, config: &OverlaySurfaceConfig) -> Result<(), String> {
        let system = self
            .system
            .as_ref()
            .ok_or_else(|| "OpenVR system interface is not started".to_string())?;
        let handle = self.surface_handle(&config.surface_id)?;
        let overlay = self
            .overlay
            .as_mut()
            .ok_or_else(|| "OpenVR overlay is not started".to_string())?;

        overlay
            .set_width(handle, config.physical_width_meters)
            .map_err(|error| format!("set overlay width failed: {error:?}"))?;
        overlay
            .set_texel_aspect(handle, 1.0)
            .map_err(|error| format!("set overlay texel aspect failed: {error:?}"))?;

        let transform_device = match resolve_device(system, &config.placement) {
            Ok(device) => {
                tracing::debug!(
                    surface_id = config.surface_id.as_str(),
                    device_index = device.0,
                    placement = ?config.placement,
                    "resolved VR overlay tracked device"
                );
                overlay
                    .set_transform_tracked_device_relative(
                        handle,
                        device,
                        &surface_transform(&config.placement),
                    )
                    .map_err(|error| format!("set overlay transform failed: {error:?}"))?;
                Some(device)
            }
            Err(error) if is_tracked_device_unavailable(&error) => {
                tracing::warn!(
                    error = %error,
                    surface_id = config.surface_id.as_str(),
                    "VR overlay surface will wait for tracked device"
                );
                None
            }
            Err(error) => return Err(error),
        };
        if let Some(surface) = self.surfaces.get_mut(&config.surface_id) {
            surface.transform_device = transform_device;
        }
        Ok(())
    }

    fn set_visibility(
        &mut self,
        surface_id: &OverlaySurfaceId,
        visible: bool,
    ) -> Result<(), String> {
        let (handle, current_visible, pending_before_show) = {
            let surface = self.surfaces.get_mut(surface_id).ok_or_else(|| {
                format!(
                    "overlay surface '{}' is not registered",
                    surface_id.as_str()
                )
            })?;
            (
                surface.handle,
                surface.visible,
                if visible && !surface.visible {
                    surface.pending_frame.take()
                } else {
                    None
                },
            )
        };
        if current_visible == visible {
            return Ok(());
        }
        if let Some(frame) = pending_before_show {
            if let Err(error) = self.upload_frame(handle, &frame) {
                if let Some(surface) = self.surfaces.get_mut(surface_id) {
                    surface.pending_frame = Some(frame);
                }
                return Err(error);
            }
        }
        let overlay = self
            .overlay
            .as_mut()
            .ok_or_else(|| "OpenVR overlay is not started".to_string())?;
        overlay
            .set_visibility(handle, visible)
            .map_err(|error| format!("set overlay visibility failed: {error:?}"))?;
        if let Some(surface) = self.surfaces.get_mut(surface_id) {
            surface.visible = visible;
            if visible {
                surface.last_visible_frame_upload_at = Some(Instant::now());
            }
        }
        if !visible {
            if let Some(surface) = self.surfaces.get_mut(surface_id) {
                surface.last_visible_frame_upload_at = None;
            }
        }
        Ok(())
    }

    fn show_with_fade(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        let (already_visible, target_alpha) = {
            let surface = self.surfaces.get(surface_id).ok_or_else(|| {
                format!(
                    "overlay surface '{}' is not registered",
                    surface_id.as_str()
                )
            })?;
            (
                surface.visible && !surface.hide_after_fade,
                surface.target_alpha,
            )
        };
        if already_visible {
            return Ok(());
        }
        self.apply_alpha(surface_id, 0.0)?;
        self.set_visibility(surface_id, true)?;
        if let Some(surface) = self.surfaces.get_mut(surface_id) {
            surface.hide_after_fade = false;
            surface.fade = Some(SurfaceFade {
                from: surface.current_alpha,
                to: target_alpha,
                started_at: Instant::now(),
            });
        }
        Ok(())
    }

    fn hide_with_fade(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        let surface = self.surfaces.get_mut(surface_id).ok_or_else(|| {
            format!(
                "overlay surface '{}' is not registered",
                surface_id.as_str()
            )
        })?;
        if !surface.visible || surface.hide_after_fade {
            return Ok(());
        }
        surface.hide_after_fade = true;
        surface.fade = Some(SurfaceFade {
            from: surface.current_alpha,
            to: 0.0,
            started_at: Instant::now(),
        });
        Ok(())
    }

    fn advance_fades(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let mut alpha_updates = Vec::new();
        let mut hide_updates = Vec::new();
        for (surface_id, surface) in &mut self.surfaces {
            let Some(fade) = surface.fade else {
                continue;
            };
            let progress = (now.saturating_duration_since(fade.started_at).as_secs_f32()
                / SURFACE_FADE_DURATION.as_secs_f32())
            .clamp(0.0, 1.0);
            let alpha = fade.from + (fade.to - fade.from) * progress;
            alpha_updates.push((surface_id.clone(), alpha));
            if progress >= 1.0 {
                surface.fade = None;
                if surface.hide_after_fade {
                    surface.hide_after_fade = false;
                    hide_updates.push(surface_id.clone());
                }
            }
        }
        for (surface_id, alpha) in alpha_updates {
            self.apply_alpha(&surface_id, alpha)?;
        }
        for surface_id in hide_updates {
            self.set_visibility(&surface_id, false)?;
        }
        Ok(())
    }

    fn apply_alpha(&mut self, surface_id: &OverlaySurfaceId, alpha: f32) -> Result<(), String> {
        let handle = self.surface_handle(surface_id)?;
        let overlay = self
            .overlay
            .as_mut()
            .ok_or_else(|| "OpenVR overlay is not started".to_string())?;
        overlay
            .set_opacity(handle, alpha)
            .map_err(|error| format!("set overlay alpha failed: {error:?}"))?;
        if let Some(surface) = self.surfaces.get_mut(surface_id) {
            surface.current_alpha = alpha;
        }
        Ok(())
    }

    fn upload_frame(&mut self, handle: OverlayHandle, frame: &RgbaFrame) -> Result<(), String> {
        let overlay = self
            .overlay
            .as_mut()
            .ok_or_else(|| "OpenVR overlay is not started".to_string())?;
        upload_raw_frame(overlay, handle, frame)
    }

    fn surface_handle(&self, surface_id: &OverlaySurfaceId) -> Result<OverlayHandle, String> {
        self.surfaces
            .get(surface_id)
            .map(|surface| surface.handle)
            .ok_or_else(|| {
                format!(
                    "overlay surface '{}' is not registered",
                    surface_id.as_str()
                )
            })
    }
}

fn init_start_error(context: &str, error: openvr::InitError) -> BackendStartError {
    let message = format!("{context}: {error:?}");
    let permanent = matches!(
        error,
        openvr::InitError::Init_InterfaceNotFound
            | openvr::InitError::Init_InvalidInterface
            | openvr::InitError::Init_InstallationNotFound
            | openvr::InitError::Init_InstallationCorrupt
            | openvr::InitError::Init_VRClientDLLNotFound
            | openvr::InitError::Init_FactoryNotFound
            | openvr::InitError::Init_PathRegistryNotFound
    );
    if permanent {
        BackendStartError::permanent(message)
    } else {
        BackendStartError::transient(message)
    }
}

fn upload_raw_frame(
    overlay: &mut Overlay,
    handle: OverlayHandle,
    frame: &RgbaFrame,
) -> Result<(), String> {
    overlay
        .set_raw_data(
            handle,
            &frame.data,
            frame.size.width as usize,
            frame.size.height as usize,
            4,
        )
        .map_err(|error| format!("set raw overlay data failed: {error:?}"))
}

fn device_button_pressed(
    system: &openvr::System,
    device: TrackedDeviceIndex,
    button: OverlayActivationButton,
) -> bool {
    let Some(state) = system.controller_state(device) else {
        return false;
    };
    let tracking_system_name = string_property(system, device, TrackingSystemName_String);
    let mask = overlay_button_mask(button, tracking_system_name.as_deref());
    state.button_pressed & mask != 0
}

fn overlay_button_mask(button: OverlayActivationButton, tracking_system_name: Option<&str>) -> u64 {
    let button_id = match button {
        OverlayActivationButton::Grip if is_oculus_tracking_system(tracking_system_name) => {
            button_id::A
        }
        OverlayActivationButton::Grip => button_id::GRIP,
        OverlayActivationButton::Menu => button_id::APPLICATION_MENU,
    };
    1u64 << button_id
}

fn is_oculus_tracking_system(value: Option<&str>) -> bool {
    value
        .map(|value| value.to_ascii_lowercase().contains("oculus"))
        .unwrap_or(false)
}

fn resolve_device(
    system: &openvr::System,
    placement: &OverlayPlacement,
) -> Result<TrackedDeviceIndex, String> {
    match placement {
        OverlayPlacement::TrackedDeviceRelative { device_hint } => {
            let role = match device_hint.as_str() {
                "right-hand" => Some(TrackedControllerRole::RightHand),
                "left-hand" => Some(TrackedControllerRole::LeftHand),
                "hmd" | "head" => return Ok(tracked_device_index::HMD),
                value if value.starts_with("hmd:") => return Ok(tracked_device_index::HMD),
                _ => return Err(format!("unknown tracked device hint '{device_hint}'")),
            };
            resolve_controller_device(system, role.unwrap())
                .ok_or_else(|| tracked_device_unavailable_error(system, device_hint))
        }
    }
}

fn resolve_controller_device(
    system: &openvr::System,
    role: TrackedControllerRole,
) -> Option<TrackedDeviceIndex> {
    system
        .tracked_device_index_for_controller_role(role)
        .or_else(|| infer_controller_device_for_role(system, role))
}

fn infer_controller_device_for_role(
    system: &openvr::System,
    role: TrackedControllerRole,
) -> Option<TrackedDeviceIndex> {
    for index in 0..MAX_TRACKED_DEVICE_COUNT {
        let device = TrackedDeviceIndex(index as u32);
        if !system.is_tracked_device_connected(device)
            || system.tracked_device_class(device) != TrackedDeviceClass::Controller
        {
            continue;
        }
        if controller_role(system, device) == Some(role) {
            return Some(device);
        }
    }
    None
}

fn controller_role(
    system: &openvr::System,
    device: TrackedDeviceIndex,
) -> Option<TrackedControllerRole> {
    let role = system.get_controller_role_for_tracked_device_index(device);
    if matches!(
        role,
        Some(TrackedControllerRole::LeftHand | TrackedControllerRole::RightHand)
    ) {
        return role;
    }
    controller_role_hint(system, device)
}

fn controller_role_hint(
    system: &openvr::System,
    device: TrackedDeviceIndex,
) -> Option<TrackedControllerRole> {
    let value = system
        .int32_tracked_device_property(device, ControllerRoleHint_Int32)
        .ok()?;
    if value == TrackedControllerRole::LeftHand as i32 {
        Some(TrackedControllerRole::LeftHand)
    } else if value == TrackedControllerRole::RightHand as i32 {
        Some(TrackedControllerRole::RightHand)
    } else {
        None
    }
}

fn is_tracked_device_unavailable(error: &str) -> bool {
    error.starts_with("tracked device '")
}

fn tracked_device_unavailable_error(system: &openvr::System, device_hint: &str) -> String {
    let left = controller_role_index(system, TrackedControllerRole::LeftHand);
    let right = controller_role_index(system, TrackedControllerRole::RightHand);
    let connected = tracked_device_diagnostics(system);
    format!(
        "tracked device '{device_hint}' is unavailable; controller_roles={{left:{left}, right:{right}}}; connected_devices=[{connected}]"
    )
}

fn controller_role_index(system: &openvr::System, role: TrackedControllerRole) -> String {
    system
        .tracked_device_index_for_controller_role(role)
        .map(|device| device.0.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn tracked_device_diagnostics(system: &openvr::System) -> String {
    let mut rows = Vec::new();
    for index in 0..MAX_TRACKED_DEVICE_COUNT {
        let device = TrackedDeviceIndex(index as u32);
        if !system.is_tracked_device_connected(device) {
            continue;
        }
        let class = system.tracked_device_class(device);
        let raw_role = system
            .get_controller_role_for_tracked_device_index(device)
            .map(|role| format!("{role:?}"))
            .unwrap_or_else(|| "none".to_string());
        let role_hint = system
            .int32_tracked_device_property(device, ControllerRoleHint_Int32)
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "none".to_string());
        let inferred_role = controller_role(system, device)
            .map(|role| format!("{role:?}"))
            .unwrap_or_else(|| "none".to_string());
        let serial =
            string_property(system, device, SerialNumber_String).unwrap_or_else(|| "-".to_string());
        let model =
            string_property(system, device, ModelNumber_String).unwrap_or_else(|| "-".to_string());
        let tracking = string_property(system, device, TrackingSystemName_String)
            .unwrap_or_else(|| "-".to_string());
        rows.push(format!(
            "{{index:{index}, class:{class:?}, role:{raw_role}, role_hint:{role_hint}, resolved_role:{inferred_role}, serial:{serial}, model:{model}, tracking:{tracking}}}"
        ));
    }
    if rows.is_empty() {
        "none".to_string()
    } else {
        rows.join(", ")
    }
}

fn surface_transform(placement: &OverlayPlacement) -> Matrix3x4 {
    match placement {
        OverlayPlacement::TrackedDeviceRelative { device_hint } if device_hint == "left-hand" => {
            Matrix3x4([
                [0.0, 0.0, -1.0, -0.07],
                [0.0, -1.0, 0.0, -0.05],
                [-1.0, 0.0, 0.0, 0.06],
            ])
        }
        OverlayPlacement::TrackedDeviceRelative { device_hint } if device_hint == "right-hand" => {
            Matrix3x4([
                [0.0, 0.0, 1.0, 0.07],
                [0.0, -1.0, 0.0, -0.05],
                [1.0, 0.0, 0.0, 0.06],
            ])
        }
        OverlayPlacement::TrackedDeviceRelative { device_hint }
            if device_hint.starts_with("hmd") =>
        {
            hmd_transform(device_hint)
        }
        OverlayPlacement::TrackedDeviceRelative { .. } => Matrix3x4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.035],
            [0.0, 0.0, 1.0, 0.055],
        ]),
    }
}

fn hmd_transform(device_hint: &str) -> Matrix3x4 {
    let (x, y) = match device_hint {
        "hmd:top" => (0.0, 0.38),
        "hmd:left" => (-0.52, -0.12),
        "hmd:right" => (0.52, -0.12),
        _ => (0.0, -0.38),
    };
    Matrix3x4([
        [1.0, 0.0, 0.0, x],
        [0.0, 1.0, 0.0, y],
        [0.0, 0.0, 1.0, -1.15],
    ])
}

fn snapshot_openvr_devices(system: &openvr::System) -> Vec<VrDeviceSnapshot> {
    let poses = system.device_to_absolute_tracking_pose(TrackingUniverseOrigin::Standing, 0.0);
    let mut rows = Vec::new();
    let mut tracker_index = 0usize;

    for index in 0..MAX_TRACKED_DEVICE_COUNT {
        let device = TrackedDeviceIndex(index as u32);
        if !system.is_tracked_device_connected(device) {
            continue;
        }
        let class = system.tracked_device_class(device);
        if !is_display_device_class(class) {
            continue;
        }

        let role = controller_role(system, device);
        let serial = string_property(system, device, SerialNumber_String);
        let model = string_property(system, device, ModelNumber_String);
        let label = match class {
            TrackedDeviceClass::HMD => "HMD".to_string(),
            TrackedDeviceClass::Controller => match role {
                Some(TrackedControllerRole::LeftHand) => "L".to_string(),
                Some(TrackedControllerRole::RightHand) => "R".to_string(),
                _ => short_device_label(model.as_deref(), serial.as_deref(), "C"),
            },
            TrackedDeviceClass::GenericTracker => {
                tracker_index += 1;
                format!("T{tracker_index}")
            }
            _ => short_device_label(model.as_deref(), serial.as_deref(), "VR"),
        };
        let battery_percent = battery_percent(system, device);
        let charging = bool_property(system, device, DeviceIsCharging_Bool).unwrap_or(false);
        let pose_valid = poses
            .get(index)
            .is_some_and(|pose| pose.device_is_connected() && pose.pose_is_valid());
        let status = device_status(battery_percent, charging, pose_valid);
        rows.push(DeviceRow {
            sort_key: device_sort_key(class, role, tracker_index),
            snapshot: VrDeviceSnapshot {
                label,
                serial,
                status,
                battery_percent,
            },
        });
    }

    rows.sort_by_key(|row| row.sort_key);
    rows.into_iter().map(|row| row.snapshot).collect()
}

struct DeviceRow {
    sort_key: (u8, usize),
    snapshot: VrDeviceSnapshot,
}

fn is_display_device_class(class: TrackedDeviceClass) -> bool {
    matches!(
        class,
        TrackedDeviceClass::HMD
            | TrackedDeviceClass::Controller
            | TrackedDeviceClass::GenericTracker
    )
}

fn device_sort_key(
    class: TrackedDeviceClass,
    role: Option<TrackedControllerRole>,
    tracker_index: usize,
) -> (u8, usize) {
    match class {
        TrackedDeviceClass::HMD => (0, 0),
        TrackedDeviceClass::Controller => match role {
            Some(TrackedControllerRole::LeftHand) => (1, 0),
            Some(TrackedControllerRole::RightHand) => (2, 0),
            _ => (3, 0),
        },
        TrackedDeviceClass::GenericTracker => (4, tracker_index),
        _ => (9, 0),
    }
}

fn string_property(
    system: &openvr::System,
    device: TrackedDeviceIndex,
    property: openvr::TrackedDeviceProperty,
) -> Option<String> {
    system
        .string_tracked_device_property(device, property)
        .ok()
        .and_then(|value| value.into_string().ok())
        .map(|value| value.trim_matches(char::from(0)).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn bool_property(
    system: &openvr::System,
    device: TrackedDeviceIndex,
    property: openvr::TrackedDeviceProperty,
) -> Option<bool> {
    system.bool_tracked_device_property(device, property).ok()
}

fn battery_percent(system: &openvr::System, device: TrackedDeviceIndex) -> Option<u8> {
    if bool_property(system, device, DeviceProvidesBatteryStatus_Bool) == Some(false) {
        return None;
    }
    system
        .float_tracked_device_property(device, DeviceBatteryPercentage_Float)
        .ok()
        .map(|value| (value.clamp(0.0, 1.0) * 100.0).round() as u8)
}

fn device_status(battery_percent: Option<u8>, charging: bool, pose_valid: bool) -> VrDeviceStatus {
    if charging {
        return VrDeviceStatus::Charging;
    }
    if !pose_valid {
        return VrDeviceStatus::TrackingWarning;
    }
    match battery_percent {
        Some(percent) if percent <= 10 => VrDeviceStatus::CriticalBattery,
        Some(percent) if percent <= 25 => VrDeviceStatus::LowBattery,
        _ => VrDeviceStatus::Normal,
    }
}

fn short_device_label(model: Option<&str>, serial: Option<&str>, fallback: &str) -> String {
    let raw = model
        .filter(|value| !value.trim().is_empty())
        .or_else(|| serial.filter(|value| !value.trim().is_empty()))
        .unwrap_or(fallback)
        .trim();
    raw.split_whitespace()
        .next()
        .unwrap_or(fallback)
        .chars()
        .take(6)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_mask_uses_oculus_a_for_grip() {
        assert_eq!(
            overlay_button_mask(OverlayActivationButton::Grip, Some("oculus")),
            1u64 << (button_id::A as u32)
        );
    }

    #[test]
    fn button_mask_uses_grip_for_non_oculus_grip() {
        assert_eq!(
            overlay_button_mask(OverlayActivationButton::Grip, Some("lighthouse")),
            1u64 << (button_id::GRIP as u32)
        );
    }

    #[test]
    fn button_mask_uses_application_menu_for_menu() {
        assert_eq!(
            overlay_button_mask(OverlayActivationButton::Menu, Some("oculus")),
            1u64 << (button_id::APPLICATION_MENU as u32)
        );
        assert_eq!(
            overlay_button_mask(OverlayActivationButton::Menu, Some("lighthouse")),
            1u64 << (button_id::APPLICATION_MENU as u32)
        );
    }
}
