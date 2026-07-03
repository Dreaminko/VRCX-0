mod font;
pub mod layout;
pub mod model;
pub mod render;
pub mod scene;
pub mod surfaces;

pub use font::{new_shared_overlay_font_system, SharedOverlayFontSystem};
pub use layout::TextMeasurer;
pub use model::{
    Color, DeviceChip, DeviceRole, DeviceStatus, FeedKind, FeedLine, FeedRelation, FeedSeverity,
    OverlayFooter, OverlaySize, OverlaySurfaceId, Rect, RgbaFrame, MAIN_SURFACE_ID,
};
pub use render::{OverlayRenderError, OverlayRenderer, TinySkiaRenderer};
pub use scene::{DrawCommand, HitRegion, OverlayScene, TextStyle};
pub use surfaces::main::{build_main_scene, AvatarBitmap, MainSurfaceModel, ToastCard};
pub use surfaces::wrist::{build_wrist_scene, WristSurfaceModel};
