use image::{codecs::png::PngEncoder, ColorType, ImageEncoder, RgbaImage};
use vrcx_0_vr_overlay::{
    default_slint_panel_size, FavoriteFriendsPanelModel, MainSurfaceModel, RgbaFrame,
    SlintHmdRenderer, SlintPanelHost, SlintPanelPointerEvent, SlintPanelRenderStats,
    SlintWristRenderer, WristSurfaceModel,
};

pub struct RenderedPng {
    pub bytes: Vec<u8>,
    pub stats: Option<SlintPanelRenderStats>,
}

pub struct DevtoolRenderer {
    wrist: SlintWristRenderer,
    hmd: SlintHmdRenderer,
    panel: Option<SlintPanelHost>,
    panel_frame: Option<RgbaFrame>,
    panel_stats: Option<SlintPanelRenderStats>,
}

impl DevtoolRenderer {
    pub fn new() -> Self {
        Self {
            wrist: SlintWristRenderer::new(),
            hmd: SlintHmdRenderer::new(),
            panel: None,
            panel_frame: None,
            panel_stats: None,
        }
    }

    pub fn friends_png(
        &mut self,
        model: &FavoriteFriendsPanelModel,
    ) -> Result<RenderedPng, String> {
        self.panel_png(model)
    }

    pub fn main_png(&mut self, model: &MainSurfaceModel) -> Result<RenderedPng, String> {
        let frame = self.hmd.render(model)?;
        frame_png(frame).map(RenderedPng::without_stats)
    }

    pub fn wrist_png(&mut self, model: &WristSurfaceModel) -> Result<RenderedPng, String> {
        let frame = self.wrist.render(model)?;
        frame_png(frame).map(RenderedPng::without_stats)
    }

    pub fn dispatch_panel_input(&mut self, event: SlintPanelPointerEvent) -> Result<(), String> {
        self.panel_host()?.dispatch(event)
    }

    pub fn reset_panel(&mut self) {
        self.panel = None;
        self.panel_frame = None;
        self.panel_stats = None;
    }

    fn panel_png(&mut self, model: &FavoriteFriendsPanelModel) -> Result<RenderedPng, String> {
        let rendered = {
            let host = self.panel_host()?;
            host.set_model(model);
            host.render_if_needed()?
        };
        if let Some(rendered) = rendered {
            self.panel_stats = Some(rendered.stats);
            self.panel_frame = Some(rendered.frame);
        }
        let frame = self
            .panel_frame
            .clone()
            .ok_or_else(|| "Slint panel did not produce a frame".to_string())?;
        Ok(RenderedPng {
            bytes: frame_png(frame)?,
            stats: self.panel_stats,
        })
    }

    fn panel_host(&mut self) -> Result<&mut SlintPanelHost, String> {
        if self.panel.is_none() {
            self.panel = Some(SlintPanelHost::new(default_slint_panel_size())?);
        }
        self.panel
            .as_mut()
            .ok_or_else(|| "Slint panel host is unavailable".to_string())
    }
}

impl RenderedPng {
    fn without_stats(bytes: Vec<u8>) -> Self {
        Self { bytes, stats: None }
    }
}

impl Default for DevtoolRenderer {
    fn default() -> Self {
        Self::new()
    }
}

const BACKDROPS: [u8; 3] = [0, 110, 235];

pub fn backdrop_sheet_png(png: &[u8]) -> Result<Vec<u8>, String> {
    let overlay = image::load_from_memory(png)
        .map_err(|error| format!("decode PNG failed: {error}"))?
        .into_rgba8();
    let (width, height) = overlay.dimensions();
    let mut sheet = RgbaImage::new(width * BACKDROPS.len() as u32, height);
    for (tile, level) in BACKDROPS.iter().enumerate() {
        let offset = tile as u32 * width;
        for (x, y, pixel) in overlay.enumerate_pixels() {
            let alpha = f32::from(pixel[3]) / 255.0;
            let blended: [u8; 3] = std::array::from_fn(|channel| {
                (f32::from(pixel[channel]) + f32::from(*level) * (1.0 - alpha)).round() as u8
            });
            sheet.put_pixel(
                offset + x,
                y,
                image::Rgba([blended[0], blended[1], blended[2], 255]),
            );
        }
    }
    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(
            sheet.as_raw(),
            sheet.width(),
            sheet.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|error| format!("encode PNG failed: {error}"))?;
    Ok(encoded)
}

pub fn frame_png(frame: RgbaFrame) -> Result<Vec<u8>, String> {
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
