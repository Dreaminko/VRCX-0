mod persistence;
mod remote;
mod service;
mod types;

pub use service::CommunityThemeService;
pub use types::{
    CommunityThemeAuthor, CommunityThemeCatalog, CommunityThemeConfigureInput,
    CommunityThemeInstallMetadata, CommunityThemeManifest, CommunityThemeProjection,
    CommunityThemeStatsById, CommunityThemeStatsEntry,
};

#[cfg(test)]
use remote::{CommunityThemeRemote, CommunityThemeRemoteFuture};
#[cfg(test)]
use vrcx_0_integrations::community_theme as protocol;

#[cfg(test)]
mod tests;
