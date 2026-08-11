#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WristOverlayStartMode {
    SteamVr,
    #[default]
    Vrchat,
}

impl WristOverlayStartMode {
    pub fn from_config(value: &str) -> Self {
        match value.trim() {
            "steamvr" => Self::SteamVr,
            _ => Self::Vrchat,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VrOverlayEligibility {
    pub enabled: bool,
    pub backend_available: bool,
    pub game_running: bool,
    pub steamvr_running: bool,
    pub start_mode: WristOverlayStartMode,
}

impl VrOverlayEligibility {
    pub fn can_run(self) -> bool {
        self.enabled
            && self.backend_available
            && self.steamvr_running
            && match self.start_mode {
                WristOverlayStartMode::SteamVr => true,
                WristOverlayStartMode::Vrchat => self.game_running,
            }
    }
}
