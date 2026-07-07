use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use vrcx_0_vr_overlay::{
    build_friends_panel_scene_with_text, build_main_scene, build_wrist_scene,
    FavoriteFriendsPanelModel, MainSurfaceModel, OverlayRenderer, OverlayScene, RgbaFrame,
    TextMeasurer, TinySkiaRenderer, WristSurfaceModel,
};

pub struct DevtoolRenderer {
    renderer: TinySkiaRenderer,
    text: TextMeasurer,
}

impl DevtoolRenderer {
    pub fn new() -> Self {
        Self {
            renderer: TinySkiaRenderer::new(),
            text: TextMeasurer::new(),
        }
    }

    pub fn friends_png(&mut self, model: &FavoriteFriendsPanelModel) -> Result<Vec<u8>, String> {
        let scene = build_friends_panel_scene_with_text(model, &mut self.text);
        self.scene_png(scene)
    }

    pub fn main_png(&mut self, model: &MainSurfaceModel) -> Result<Vec<u8>, String> {
        let scene = build_main_scene(model, &mut self.text);
        self.scene_png(scene)
    }

    pub fn wrist_png(&mut self, model: &WristSurfaceModel) -> Result<Vec<u8>, String> {
        let scene = build_wrist_scene(model, &mut self.text);
        self.scene_png(scene)
    }

    fn scene_png(&mut self, scene: OverlayScene) -> Result<Vec<u8>, String> {
        let frame = self
            .renderer
            .render(&scene)
            .map_err(|error| error.to_string())?;
        frame_png(frame)
    }
}

impl Default for DevtoolRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn frame_png(frame: RgbaFrame) -> Result<Vec<u8>, String> {
    if !frame.is_valid_len() {
        return Err(format!(
            "invalid frame length for {}x{}",
            frame.size.width, frame.size.height
        ));
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(
            &frame.data,
            frame.size.width,
            frame.size.height,
            ColorType::Rgba8.into(),
        )
        .map_err(|error| format!("encode PNG failed: {error}"))?;
    Ok(png)
}
