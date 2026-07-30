use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Notify;
use vrcx_0_application_core::sleep_until_due_or_stopped;
use vrcx_0_application_core::RuntimeEventBus;
use vrcx_0_application_core::TaskStopToken;
use vrcx_0_application_core::WebClient;
use vrcx_0_core::json::text_of;
use vrcx_0_core::time::now_iso;
use vrcx_0_integrations::background_image as protocol;
use vrcx_0_integrations::external_api::{self, ExternalApiScope};
use vrcx_0_persistence::{config as config_store, DatabaseService};

use crate::{Error, Result};

const KEY_ENABLED: &str = "VRCX_backgroundImageEnabled";
const KEY_MODE: &str = "VRCX_backgroundImageMode";
const KEY_PROVIDER_ID: &str = "VRCX_backgroundImageProviderId";
const KEY_SNAPSHOTS: &str = "VRCX_backgroundImageSnapshots";
const KEY_CUSTOM_SOURCE: &str = "VRCX_backgroundImageCustomSource";
const KEY_LEGACY_ENABLED: &str = "VRCX_officialBackgroundEnabled";
const KEY_LEGACY_PROVIDER_ID: &str = "VRCX_officialBackgroundProviderId";
const KEY_LEGACY_SNAPSHOTS: &str = "VRCX_officialBackgroundSnapshots";
const KEY_COMMUNITY_THEME_ENABLED: &str = "VRCX_communityThemeEnabled";
const KEY_COMMUNITY_THEME_INSTALLED_THEMES: &str = "VRCX_communityThemeInstalledThemes";
const KEY_COMMUNITY_THEME_INSTALL_METADATA: &str = "VRCX_communityThemeInstallMetadata";
const KEY_COMMUNITY_THEME_CSS_SNAPSHOT: &str = "VRCX_communityThemeCssSnapshot";

const SNAPSHOT_TTL_HOURS: i64 = 24;
const ROTATION_BOUNDARY_GRACE_SECONDS: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundImageMode {
    Off,
    Daily,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BackgroundImageProviderId {
    NasaEpic,
    AicPublicDomain,
    NasaApodSafe,
}

impl BackgroundImageProviderId {
    pub const ALL: [Self; 3] = [Self::NasaEpic, Self::AicPublicDomain, Self::NasaApodSafe];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NasaEpic => "nasa-epic",
            Self::AicPublicDomain => "aic-public-domain",
            Self::NasaApodSafe => "nasa-apod-safe",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::NasaEpic => "NASA EPIC",
            Self::AicPublicDomain => "Art Institute of Chicago",
            Self::NasaApodSafe => "NASA APOD",
        }
    }

    fn from_config(value: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|provider| provider.as_str() == value.trim())
            .unwrap_or(Self::NasaEpic)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundImageRotationInterval {
    Daily,
    Hourly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundImageCustomSourceKind {
    Files,
    Folder,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImageCustomSource {
    pub kind: BackgroundImageCustomSourceKind,
    pub paths: Vec<String>,
    pub folder_path: String,
    pub rotation_interval: BackgroundImageRotationInterval,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImageSnapshot {
    pub mode: BackgroundImageMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<BackgroundImageProviderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<BackgroundImageCustomSourceKind>,
    pub image_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_count: Option<u32>,
    pub title: String,
    pub author: String,
    pub license: String,
    pub source: String,
    pub resolved_at: String,
    pub resolved_for_key: String,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImageProjection {
    pub revision: u64,
    pub enabled: bool,
    pub mode: BackgroundImageMode,
    pub provider_id: BackgroundImageProviderId,
    pub custom_source: Option<BackgroundImageCustomSource>,
    pub snapshot: Option<BackgroundImageSnapshot>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BackgroundImageConfigureInput {
    Disable,
    #[serde(rename_all = "camelCase")]
    EnableDaily {
        provider_id: Option<BackgroundImageProviderId>,
    },
    #[serde(rename_all = "camelCase")]
    SetProvider {
        provider_id: BackgroundImageProviderId,
    },
    EnableCustom,
    #[serde(rename_all = "camelCase")]
    SetCustomFiles {
        paths: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    SetCustomFolder {
        folder_path: String,
    },
    #[serde(rename_all = "camelCase")]
    SetRotationInterval {
        rotation_interval: BackgroundImageRotationInterval,
    },
    MigrateLegacyNasaApod,
}

pub trait BackgroundImageFileResolver: Send + Sync {
    fn resolve_files(&self, source: &BackgroundImageCustomSource) -> Result<Vec<String>>;
}

pub struct UnavailableBackgroundImageFileResolver;

impl BackgroundImageFileResolver for UnavailableBackgroundImageFileResolver {
    fn resolve_files(&self, _source: &BackgroundImageCustomSource) -> Result<Vec<String>> {
        Err(Error::Custom(
            "Custom background image sources are unavailable on this host.".into(),
        ))
    }
}

struct BackgroundImageServiceInner {
    db: Arc<DatabaseService>,
    web: Arc<WebClient>,
    event_bus: RuntimeEventBus,
    resolver: Arc<dyn BackgroundImageFileResolver>,
    projection: Mutex<BackgroundImageProjection>,
    generation: AtomicU64,
    revision: AtomicU64,
    rotation_notify: Notify,
}

#[derive(Clone)]
pub struct BackgroundImageService {
    inner: Arc<BackgroundImageServiceInner>,
}

impl BackgroundImageService {
    pub fn new(
        db: Arc<DatabaseService>,
        web: Arc<WebClient>,
        event_bus: RuntimeEventBus,
        resolver: Arc<dyn BackgroundImageFileResolver>,
    ) -> Self {
        Self {
            inner: Arc::new(BackgroundImageServiceInner {
                db,
                web,
                event_bus,
                resolver,
                projection: Mutex::new(BackgroundImageProjection {
                    revision: 0,
                    enabled: false,
                    mode: BackgroundImageMode::Off,
                    provider_id: BackgroundImageProviderId::NasaEpic,
                    custom_source: None,
                    snapshot: None,
                    error: None,
                }),
                generation: AtomicU64::new(0),
                revision: AtomicU64::new(0),
                rotation_notify: Notify::new(),
            }),
        }
    }

    pub fn projection(&self) -> BackgroundImageProjection {
        self.inner.projection.lock().unwrap().clone()
    }

    fn begin_operation(&self) -> u64 {
        self.inner
            .generation
            .fetch_add(1, AtomicOrdering::AcqRel)
            .saturating_add(1)
    }

    fn is_current_operation(&self, operation: u64) -> bool {
        self.inner.generation.load(AtomicOrdering::Acquire) == operation
    }

    fn next_revision(&self) -> u64 {
        self.inner
            .revision
            .fetch_add(1, AtomicOrdering::AcqRel)
            .saturating_add(1)
    }

    fn apply_projection(
        &self,
        operation: u64,
        mut projection: BackgroundImageProjection,
        persist: impl FnOnce(&Self, &BackgroundImageProjection) -> Result<()>,
    ) -> Result<BackgroundImageProjection> {
        let mut slot = self.inner.projection.lock().unwrap();
        if !self.is_current_operation(operation) {
            return Ok(slot.clone());
        }
        persist(self, &projection)?;
        projection.revision = self.next_revision();
        *slot = projection.clone();
        drop(slot);
        self.inner.event_bus.emit(projection.clone());
        self.inner.rotation_notify.notify_waiters();
        Ok(projection)
    }

    fn persist_state(&self, projection: &BackgroundImageProjection) -> Result<()> {
        config_store::set_bool(&self.inner.db, KEY_ENABLED, projection.enabled)?;
        config_store::set_string(&self.inner.db, KEY_MODE, mode_as_str(projection.mode))?;
        config_store::set_string(
            &self.inner.db,
            KEY_PROVIDER_ID,
            projection.provider_id.as_str(),
        )?;
        Ok(())
    }

    fn persist_custom_source(&self, source: Option<&BackgroundImageCustomSource>) -> Result<()> {
        match source {
            Some(source) => config_store::set_json(
                &self.inner.db,
                KEY_CUSTOM_SOURCE,
                &serde_json::to_value(source).map_err(|error| Error::Custom(error.to_string()))?,
            ),
            None => config_store::remove(&self.inner.db, KEY_CUSTOM_SOURCE),
        }
        .map_err(Error::from)
    }

    fn load_custom_source(&self) -> Result<Option<BackgroundImageCustomSource>> {
        let value = config_store::get_json(&self.inner.db, KEY_CUSTOM_SOURCE, Value::Null)?;
        Ok(normalize_custom_source(&value))
    }

    fn load_snapshots(&self) -> Result<Value> {
        let current = config_store::get_raw(&self.inner.db, KEY_SNAPSHOTS)?;
        let raw = match current {
            Some(raw) => raw,
            None => config_store::get_string(&self.inner.db, KEY_LEGACY_SNAPSHOTS, "{}")?,
        };
        Ok(serde_json::from_str(&raw).unwrap_or_else(|_| json!({})))
    }

    fn cached_provider_snapshot(
        &self,
        provider_id: BackgroundImageProviderId,
    ) -> Result<Option<BackgroundImageSnapshot>> {
        let snapshots = self.load_snapshots()?;
        Ok(normalize_provider_snapshot(
            snapshots.get(provider_id.as_str()),
            provider_id,
        ))
    }

    async fn fetch_provider_json(&self, url: &str) -> Result<(i32, String)> {
        let response = self
            .inner
            .web
            .execute_external_api(
                external_api::background_image_get_input(url),
                ExternalApiScope::BackgroundImage,
            )
            .await?;
        Ok((response.status, response.data))
    }

    async fn fetch_provider_image(
        &self,
        provider_id: BackgroundImageProviderId,
    ) -> Result<protocol::BackgroundImageProviderImage> {
        let date_key = current_utc_date_key();
        match provider_id {
            BackgroundImageProviderId::NasaEpic => {
                let (status, body) = self
                    .fetch_provider_json(protocol::NASA_EPIC_METADATA_URL)
                    .await?;
                ensure_provider_status(status)?;
                protocol::parse_nasa_epic_response(&body)
                    .map_err(|error| Error::Custom(error.to_string()))
            }
            BackgroundImageProviderId::AicPublicDomain => {
                let (status, body) = self
                    .fetch_provider_json(protocol::AIC_PUBLIC_DOMAIN_SEARCH_URL)
                    .await?;
                ensure_provider_status(status)?;
                protocol::parse_aic_response(&body, &date_key)
                    .map_err(|error| Error::Custom(error.to_string()))
            }
            BackgroundImageProviderId::NasaApodSafe => {
                let today = Utc::now();
                for offset in 0..=protocol::NASA_APOD_IMAGE_LOOKBACK_DAYS {
                    let date = (today - chrono::Duration::days(offset as i64))
                        .format("%Y-%m-%d")
                        .to_string();
                    let (status, body) = self
                        .fetch_provider_json(&protocol::nasa_apod_request_url(&date))
                        .await?;
                    if status == 404 {
                        continue;
                    }
                    ensure_provider_status(status)?;
                    if let Some(image) = protocol::parse_nasa_apod_response(&body, &date_key)
                        .map_err(|error| Error::Custom(error.to_string()))?
                    {
                        return Ok(image);
                    }
                }
                Err(Error::Custom(
                    "NASA APOD did not return a copyright-free image in the recent archive.".into(),
                ))
            }
        }
    }

    async fn resolve_provider_snapshot(
        &self,
        provider_id: BackgroundImageProviderId,
        force_refresh: bool,
    ) -> Result<Option<BackgroundImageSnapshot>> {
        let mut snapshots = self.load_snapshots()?;
        let cached = normalize_provider_snapshot(snapshots.get(provider_id.as_str()), provider_id);
        if !force_refresh && is_snapshot_fresh(cached.as_ref()) {
            return Ok(cached);
        }

        match self.fetch_provider_image(provider_id).await {
            Ok(image) => {
                let snapshot = BackgroundImageSnapshot {
                    mode: BackgroundImageMode::Daily,
                    provider_id: Some(provider_id),
                    source_kind: None,
                    image_url: image.image_url,
                    image_path: None,
                    image_count: None,
                    title: image.title,
                    author: image.author,
                    license: image.license,
                    source: image.source,
                    resolved_at: now_iso(),
                    resolved_for_key: current_utc_date_key(),
                };
                if !snapshots.is_object() {
                    snapshots = json!({});
                }
                snapshots[provider_id.as_str()] = serde_json::to_value(&snapshot)
                    .map_err(|error| Error::Custom(error.to_string()))?;
                config_store::set_json(&self.inner.db, KEY_SNAPSHOTS, &snapshots)?;
                Ok(Some(snapshot))
            }
            Err(error) => {
                if cached.is_some() {
                    tracing::warn!(
                        provider = provider_id.as_str(),
                        error = %error,
                        "unable to refresh background image; using cached snapshot"
                    );
                    Ok(cached)
                } else {
                    Err(error)
                }
            }
        }
    }

    fn resolve_custom_snapshot(
        &self,
        source: &BackgroundImageCustomSource,
        previous: Option<&BackgroundImageSnapshot>,
    ) -> Result<BackgroundImageSnapshot> {
        let files = self.inner.resolver.resolve_files(source)?;
        assert_selected_files_available(source, &files)?;
        assert_previous_image_available(source, &files, previous)?;
        if files.is_empty() {
            return Err(Error::Custom(
                "No supported images were found in the selected source.".into(),
            ));
        }

        let (key, index) = if files.len() <= 1 {
            ("static".to_string(), 0)
        } else {
            let key = rotation_key(source.rotation_interval, Local::now());
            let index =
                (stable_hash(&format!("{}:{key}", source_hash_key(source))) as usize) % files.len();
            (key, index)
        };
        let image_path = files[index].clone();
        let title = file_name_from_path(&image_path);

        Ok(BackgroundImageSnapshot {
            mode: BackgroundImageMode::Custom,
            provider_id: None,
            source_kind: Some(source.kind),
            image_url: String::new(),
            image_path: Some(image_path),
            image_count: Some(files.len() as u32),
            title,
            author: "Custom image source".into(),
            license: "Local file".into(),
            source: match source.kind {
                BackgroundImageCustomSourceKind::Folder => source.folder_path.clone(),
                BackgroundImageCustomSourceKind::Files => {
                    let count = files.len();
                    format!(
                        "{count} selected image{}",
                        if count == 1 { "" } else { "s" }
                    )
                }
            },
            resolved_at: now_iso(),
            resolved_for_key: key,
        })
    }

    pub async fn initialize(&self) -> Result<BackgroundImageProjection> {
        let operation = self.begin_operation();
        let db = &self.inner.db;
        let legacy_enabled = config_store::get_bool(db, KEY_LEGACY_ENABLED, false)?;
        let enabled = config_store::get_bool(db, KEY_ENABLED, legacy_enabled)?;
        let mode = normalize_mode(&config_store::get_string(
            db,
            KEY_MODE,
            if enabled { "daily" } else { "off" },
        )?);
        let legacy_provider = config_store::get_string(db, KEY_LEGACY_PROVIDER_ID, "nasa-epic")?;
        let provider_id = BackgroundImageProviderId::from_config(&config_store::get_string(
            db,
            KEY_PROVIDER_ID,
            &legacy_provider,
        )?);
        let custom_source = self.load_custom_source()?;
        let community_active = community_theme_appearance_active(db)?;

        let mut next_enabled = enabled && mode != BackgroundImageMode::Off;
        let mut next_mode = mode;
        let mut snapshot: Option<BackgroundImageSnapshot> = None;

        if next_enabled && mode == BackgroundImageMode::Daily {
            snapshot = match self.resolve_provider_snapshot(provider_id, false).await {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    tracing::warn!(error = %error, "unable to initialize background image");
                    None
                }
            };
            next_enabled = snapshot.is_some() && !community_active;
        } else if next_enabled && mode == BackgroundImageMode::Custom {
            snapshot = match custom_source
                .as_ref()
                .map(|source| self.resolve_custom_snapshot(source, None))
            {
                Some(Ok(snapshot)) => Some(snapshot),
                Some(Err(error)) => {
                    tracing::warn!(error = %error, "unable to initialize custom background image");
                    None
                }
                None => None,
            };
            if snapshot.is_none() || community_active {
                next_enabled = false;
                next_mode = BackgroundImageMode::Off;
            }
        } else {
            next_mode = if mode == BackgroundImageMode::Custom {
                BackgroundImageMode::Custom
            } else {
                BackgroundImageMode::Off
            };
        }

        let projection = BackgroundImageProjection {
            revision: 0,
            enabled: next_enabled,
            mode: next_mode,
            provider_id,
            custom_source,
            snapshot,
            error: None,
        };
        self.apply_projection(operation, projection, Self::persist_state)
    }

    pub async fn configure(
        &self,
        input: BackgroundImageConfigureInput,
    ) -> Result<BackgroundImageProjection> {
        match input {
            BackgroundImageConfigureInput::Disable => self.disable(),
            BackgroundImageConfigureInput::EnableDaily { provider_id } => {
                self.enable_daily(provider_id).await
            }
            BackgroundImageConfigureInput::SetProvider { provider_id } => {
                self.set_provider(provider_id).await
            }
            BackgroundImageConfigureInput::EnableCustom => self.enable_custom(None),
            BackgroundImageConfigureInput::SetCustomFiles { paths } => {
                let rotation_interval = self.current_rotation_interval();
                let source = files_source(paths, rotation_interval);
                self.enable_custom(Some(source))
            }
            BackgroundImageConfigureInput::SetCustomFolder { folder_path } => {
                let rotation_interval = self.current_rotation_interval();
                let source = folder_source(folder_path, rotation_interval);
                self.enable_custom(Some(source))
            }
            BackgroundImageConfigureInput::SetRotationInterval { rotation_interval } => {
                self.set_rotation_interval(rotation_interval)
            }
            BackgroundImageConfigureInput::MigrateLegacyNasaApod => self.migrate_legacy_nasa_apod(),
        }
    }

    fn current_rotation_interval(&self) -> BackgroundImageRotationInterval {
        self.projection()
            .custom_source
            .map(|source| source.rotation_interval)
            .unwrap_or(BackgroundImageRotationInterval::Daily)
    }

    fn disable(&self) -> Result<BackgroundImageProjection> {
        let operation = self.begin_operation();
        let mut projection = self.projection();
        projection.enabled = false;
        projection.mode = BackgroundImageMode::Off;
        projection.error = None;
        self.apply_projection(operation, projection, Self::persist_state)
    }

    async fn enable_daily(
        &self,
        provider_id: Option<BackgroundImageProviderId>,
    ) -> Result<BackgroundImageProjection> {
        let operation = self.begin_operation();
        let current = self.projection();
        let provider_id = provider_id.unwrap_or(current.provider_id);
        match self.resolve_provider_snapshot(provider_id, false).await {
            Ok(snapshot) => {
                let enabled = snapshot.is_some();
                let projection = BackgroundImageProjection {
                    enabled,
                    mode: if enabled {
                        BackgroundImageMode::Daily
                    } else {
                        BackgroundImageMode::Off
                    },
                    provider_id,
                    snapshot,
                    error: None,
                    ..current
                };
                self.apply_projection(operation, projection, Self::persist_state)
            }
            Err(error) => {
                self.record_error(operation, &error);
                Err(error)
            }
        }
    }

    async fn set_provider(
        &self,
        provider_id: BackgroundImageProviderId,
    ) -> Result<BackgroundImageProjection> {
        let current = self.projection();
        if current.provider_id == provider_id {
            return Ok(current);
        }

        if current.enabled && current.mode == BackgroundImageMode::Daily {
            return self.enable_daily(Some(provider_id)).await;
        }

        let operation = self.begin_operation();
        let mut current = current;
        let snapshot = if current
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.provider_id == Some(provider_id))
        {
            current.snapshot.take()
        } else {
            self.cached_provider_snapshot(provider_id)?
        };
        let projection = BackgroundImageProjection {
            provider_id,
            snapshot,
            error: None,
            ..current
        };
        self.apply_projection(operation, projection, |service, projection| {
            config_store::set_string(
                &service.inner.db,
                KEY_PROVIDER_ID,
                projection.provider_id.as_str(),
            )
            .map_err(Error::from)
        })
    }

    fn enable_custom(
        &self,
        source: Option<BackgroundImageCustomSource>,
    ) -> Result<BackgroundImageProjection> {
        let operation = self.begin_operation();
        let current = self.projection();
        let source = source
            .and_then(normalize_custom_source_struct)
            .or(current.custom_source.clone());
        let source = match source {
            Some(source) => source,
            None => {
                let projection = BackgroundImageProjection {
                    enabled: false,
                    mode: BackgroundImageMode::Custom,
                    custom_source: None,
                    snapshot: None,
                    error: None,
                    ..current
                };
                return self.apply_projection(operation, projection, Self::persist_state);
            }
        };

        match self.resolve_custom_snapshot(&source, None) {
            Ok(snapshot) => {
                let projection = BackgroundImageProjection {
                    enabled: true,
                    mode: BackgroundImageMode::Custom,
                    custom_source: Some(source),
                    snapshot: Some(snapshot),
                    error: None,
                    ..current
                };
                self.apply_projection(operation, projection, Self::persist_state_and_source)
            }
            Err(error) => {
                self.apply_custom_failure(operation, current, Some(source), &error)?;
                Err(error)
            }
        }
    }

    fn persist_state_and_source(&self, projection: &BackgroundImageProjection) -> Result<()> {
        self.persist_custom_source(projection.custom_source.as_ref())?;
        self.persist_state(projection)
    }

    fn apply_custom_failure(
        &self,
        operation: u64,
        current: BackgroundImageProjection,
        custom_source: Option<BackgroundImageCustomSource>,
        error: &Error,
    ) -> Result<()> {
        let projection = BackgroundImageProjection {
            enabled: false,
            mode: BackgroundImageMode::Off,
            custom_source: custom_source.or(current.custom_source.clone()),
            snapshot: None,
            error: Some(error.to_string()),
            ..current
        };
        self.apply_projection(operation, projection, Self::persist_state_and_source)?;
        Ok(())
    }

    fn set_rotation_interval(
        &self,
        rotation_interval: BackgroundImageRotationInterval,
    ) -> Result<BackgroundImageProjection> {
        let current = self.projection();
        let Some(mut source) = current.custom_source.clone() else {
            return Ok(current);
        };
        source.rotation_interval = rotation_interval;
        if current.enabled && current.mode == BackgroundImageMode::Custom {
            return self.enable_custom(Some(source));
        }

        let operation = self.begin_operation();
        let projection = BackgroundImageProjection {
            custom_source: Some(source),
            ..current
        };
        self.apply_projection(operation, projection, |service, projection| {
            service.persist_custom_source(projection.custom_source.as_ref())
        })
    }

    fn migrate_legacy_nasa_apod(&self) -> Result<BackgroundImageProjection> {
        let operation = self.begin_operation();
        let mut current = self.projection();
        let snapshot = current.snapshot.take().filter(|snapshot| {
            snapshot.provider_id == Some(BackgroundImageProviderId::NasaApodSafe)
        });
        let projection = BackgroundImageProjection {
            enabled: true,
            mode: BackgroundImageMode::Daily,
            provider_id: BackgroundImageProviderId::NasaApodSafe,
            snapshot,
            error: None,
            ..current
        };
        self.apply_projection(operation, projection, Self::persist_state)
    }

    pub async fn refresh(&self, force: bool) -> Result<BackgroundImageProjection> {
        let operation = self.begin_operation();
        let current = self.projection();
        let resolved = match current.mode {
            BackgroundImageMode::Custom => match current.custom_source.as_ref() {
                Some(source) => self
                    .resolve_custom_snapshot(source, current.snapshot.as_ref())
                    .map(Some),
                None => Ok(None),
            },
            _ => {
                self.resolve_provider_snapshot(current.provider_id, force)
                    .await
            }
        };

        match resolved {
            Ok(Some(snapshot)) => {
                let mode = if current.mode == BackgroundImageMode::Custom {
                    BackgroundImageMode::Custom
                } else {
                    BackgroundImageMode::Daily
                };
                let projection = BackgroundImageProjection {
                    enabled: true,
                    mode,
                    snapshot: Some(snapshot),
                    error: None,
                    ..current
                };
                self.apply_projection(operation, projection, Self::persist_state)
            }
            Ok(None) => self.disable(),
            Err(error) => {
                if current.mode == BackgroundImageMode::Custom {
                    self.apply_custom_failure(operation, current, None, &error)?;
                } else {
                    self.record_error(operation, &error);
                }
                Err(error)
            }
        }
    }

    fn record_error(&self, operation: u64, error: &Error) {
        let mut slot = self.inner.projection.lock().unwrap();
        if !self.is_current_operation(operation) {
            return;
        }
        slot.error = Some(error.to_string());
        slot.revision = self.next_revision();
        let projection = slot.clone();
        drop(slot);
        self.inner.event_bus.emit(projection);
    }

    fn next_rotation_delay(&self) -> Option<Duration> {
        let projection = self.projection();
        if !projection.enabled || projection.mode != BackgroundImageMode::Custom {
            return None;
        }
        let source = projection.custom_source.as_ref()?;
        let rotating = match projection.snapshot.as_ref().and_then(|s| s.image_count) {
            Some(count) => count > 1,
            None => {
                source.kind == BackgroundImageCustomSourceKind::Folder || source.paths.len() > 1
            }
        };
        if !rotating {
            return None;
        }
        Some(duration_until_next_rotation(
            source.rotation_interval,
            Local::now(),
        ))
    }

    pub async fn run_rotation_loop(&self, stop_token: TaskStopToken) {
        loop {
            if stop_token.is_stop_requested() {
                return;
            }
            let notified = self.inner.rotation_notify.notified();
            match self.next_rotation_delay() {
                Some(delay) => {
                    let due = tokio::select! {
                        due = sleep_until_due_or_stopped(delay, &stop_token) => due,
                        _ = notified => false,
                    };
                    if due {
                        if let Err(error) = self.refresh(false).await {
                            tracing::warn!(error = %error, "failed to rotate background image");
                        }
                    }
                }
                None => {
                    notified.await;
                }
            }
        }
    }
}

fn community_theme_appearance_active(db: &DatabaseService) -> Result<bool> {
    if !config_store::get_bool(db, KEY_COMMUNITY_THEME_ENABLED, false)? {
        return Ok(false);
    }
    let records = config_store::get_json(db, KEY_COMMUNITY_THEME_INSTALLED_THEMES, Value::Null)?;
    if records
        .as_array()
        .is_some_and(|records| !records.is_empty())
    {
        return Ok(true);
    }
    let metadata = config_store::get_json(db, KEY_COMMUNITY_THEME_INSTALL_METADATA, Value::Null)?;
    if !text_of(metadata.get("themeId")).trim().is_empty() {
        return Ok(true);
    }
    Ok(
        !config_store::get_string(db, KEY_COMMUNITY_THEME_CSS_SNAPSHOT, "")?
            .trim()
            .is_empty(),
    )
}

fn ensure_provider_status(status: i32) -> Result<()> {
    if status == 429 {
        return Err(Error::Custom(
            "Background Image provider rate limit reached.".into(),
        ));
    }
    if !(200..300).contains(&status) {
        return Err(Error::Custom(format!(
            "Failed to load Background Image provider: {status}"
        )));
    }
    Ok(())
}

fn mode_as_str(mode: BackgroundImageMode) -> &'static str {
    match mode {
        BackgroundImageMode::Off => "off",
        BackgroundImageMode::Daily => "daily",
        BackgroundImageMode::Custom => "custom",
    }
}

fn normalize_mode(value: &str) -> BackgroundImageMode {
    match value.trim() {
        "daily" => BackgroundImageMode::Daily,
        "custom" => BackgroundImageMode::Custom,
        _ => BackgroundImageMode::Off,
    }
}

fn normalize_provider_snapshot(
    value: Option<&Value>,
    expected_provider: BackgroundImageProviderId,
) -> Option<BackgroundImageSnapshot> {
    let value = value?;
    if !value.is_object() {
        return None;
    }
    let provider_id = BackgroundImageProviderId::from_config(&text_of(value.get("providerId")));
    if provider_id != expected_provider {
        return None;
    }
    let image_url = text_of(value.get("imageUrl")).trim().to_string();
    if image_url.is_empty() {
        return None;
    }
    let resolved_for_key = {
        let key = text_of(value.get("resolvedForKey"));
        if key.is_empty() {
            text_of(value.get("resolvedForDate"))
        } else {
            key
        }
    };

    Some(BackgroundImageSnapshot {
        mode: BackgroundImageMode::Daily,
        provider_id: Some(provider_id),
        source_kind: None,
        image_url,
        image_path: None,
        image_count: None,
        title: text_of(value.get("title")),
        author: text_of(value.get("author")),
        license: text_of(value.get("license")),
        source: text_of(value.get("source")),
        resolved_at: text_of(value.get("resolvedAt")),
        resolved_for_key,
    })
}

fn is_snapshot_fresh(snapshot: Option<&BackgroundImageSnapshot>) -> bool {
    let Some(snapshot) = snapshot else {
        return false;
    };
    if snapshot.provider_id.is_none() || snapshot.resolved_at.is_empty() {
        return false;
    }
    let Ok(resolved_at) = DateTime::parse_from_rfc3339(&snapshot.resolved_at) else {
        return false;
    };
    let age = Utc::now().signed_duration_since(resolved_at.with_timezone(&Utc));
    age >= chrono::Duration::zero() && age < chrono::Duration::hours(SNAPSHOT_TTL_HOURS)
}

fn unique_paths(paths: &[String]) -> Vec<String> {
    let mut seen = Vec::new();
    for path in paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !seen.iter().any(|existing: &String| existing == trimmed) {
            seen.push(trimmed.to_string());
        }
    }
    seen
}

fn normalize_custom_source_struct(
    source: BackgroundImageCustomSource,
) -> Option<BackgroundImageCustomSource> {
    let paths = unique_paths(&source.paths);
    let folder_path = source.folder_path.trim().to_string();
    match source.kind {
        BackgroundImageCustomSourceKind::Folder if folder_path.is_empty() => None,
        BackgroundImageCustomSourceKind::Files if paths.is_empty() => None,
        BackgroundImageCustomSourceKind::Folder => Some(BackgroundImageCustomSource {
            kind: BackgroundImageCustomSourceKind::Folder,
            paths: Vec::new(),
            folder_path,
            rotation_interval: source.rotation_interval,
        }),
        BackgroundImageCustomSourceKind::Files => Some(BackgroundImageCustomSource {
            kind: BackgroundImageCustomSourceKind::Files,
            paths,
            folder_path: String::new(),
            rotation_interval: source.rotation_interval,
        }),
    }
}

fn normalize_custom_source(value: &Value) -> Option<BackgroundImageCustomSource> {
    if !value.is_object() {
        return None;
    }
    let kind = if text_of(value.get("kind")) == "folder" {
        BackgroundImageCustomSourceKind::Folder
    } else {
        BackgroundImageCustomSourceKind::Files
    };
    let paths: Vec<String> = value
        .get("paths")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| text_of(Some(entry)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let rotation_interval = if text_of(value.get("rotationInterval")) == "hourly" {
        BackgroundImageRotationInterval::Hourly
    } else {
        BackgroundImageRotationInterval::Daily
    };

    normalize_custom_source_struct(BackgroundImageCustomSource {
        kind,
        paths,
        folder_path: text_of(value.get("folderPath")),
        rotation_interval,
    })
}

fn files_source(
    paths: Vec<String>,
    rotation_interval: BackgroundImageRotationInterval,
) -> BackgroundImageCustomSource {
    BackgroundImageCustomSource {
        kind: BackgroundImageCustomSourceKind::Files,
        paths: unique_paths(&paths),
        folder_path: String::new(),
        rotation_interval,
    }
}

fn folder_source(
    folder_path: String,
    rotation_interval: BackgroundImageRotationInterval,
) -> BackgroundImageCustomSource {
    BackgroundImageCustomSource {
        kind: BackgroundImageCustomSourceKind::Folder,
        paths: Vec::new(),
        folder_path: folder_path.trim().to_string(),
        rotation_interval,
    }
}

fn path_key(path: &str) -> String {
    path.trim().to_lowercase()
}

fn assert_selected_files_available(
    source: &BackgroundImageCustomSource,
    files: &[String],
) -> Result<()> {
    if source.kind != BackgroundImageCustomSourceKind::Files {
        return Ok(());
    }
    let available: Vec<String> = files.iter().map(|file| path_key(file)).collect();
    if source
        .paths
        .iter()
        .any(|path| !available.contains(&path_key(path)))
    {
        return Err(Error::Custom(
            "A selected background image is no longer available.".into(),
        ));
    }
    Ok(())
}

fn assert_previous_image_available(
    source: &BackgroundImageCustomSource,
    files: &[String],
    previous: Option<&BackgroundImageSnapshot>,
) -> Result<()> {
    let Some(previous) = previous else {
        return Ok(());
    };
    let Some(image_path) = previous.image_path.as_deref().filter(|p| !p.is_empty()) else {
        return Ok(());
    };
    if previous.mode != BackgroundImageMode::Custom || previous.source_kind != Some(source.kind) {
        return Ok(());
    }
    if !files
        .iter()
        .any(|file| path_key(file) == path_key(image_path))
    {
        return Err(Error::Custom(
            "The current background image is no longer available.".into(),
        ));
    }
    Ok(())
}

fn source_hash_key(source: &BackgroundImageCustomSource) -> String {
    match source.kind {
        BackgroundImageCustomSourceKind::Folder => format!("folder:{}", source.folder_path),
        BackgroundImageCustomSourceKind::Files => format!("files:{}", source.paths.join("|")),
    }
}

fn stable_hash(value: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for unit in value.encode_utf16() {
        hash ^= unit as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

fn file_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn current_utc_date_key() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn rotation_key(interval: BackgroundImageRotationInterval, now: DateTime<Local>) -> String {
    match interval {
        BackgroundImageRotationInterval::Hourly => now.format("%Y-%m-%dT%H").to_string(),
        BackgroundImageRotationInterval::Daily => now.format("%Y-%m-%d").to_string(),
    }
}

fn duration_until_next_rotation(
    interval: BackgroundImageRotationInterval,
    now: DateTime<Local>,
) -> Duration {
    let next = match interval {
        BackgroundImageRotationInterval::Hourly => {
            let base = now + chrono::Duration::hours(1);
            Local
                .with_ymd_and_hms(
                    base.year(),
                    base.month(),
                    base.day(),
                    base.hour(),
                    0,
                    ROTATION_BOUNDARY_GRACE_SECONDS,
                )
                .earliest()
        }
        BackgroundImageRotationInterval::Daily => {
            let base = now + chrono::Duration::days(1);
            Local
                .with_ymd_and_hms(
                    base.year(),
                    base.month(),
                    base.day(),
                    0,
                    0,
                    ROTATION_BOUNDARY_GRACE_SECONDS,
                )
                .earliest()
        }
    };
    let millis = next
        .map(|next| (next - now).num_milliseconds())
        .unwrap_or(3_600_000)
        .max(1_000);
    Duration::from_millis(millis as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files_custom_source(paths: &[&str]) -> BackgroundImageCustomSource {
        BackgroundImageCustomSource {
            kind: BackgroundImageCustomSourceKind::Files,
            paths: paths.iter().map(|p| p.to_string()).collect(),
            folder_path: String::new(),
            rotation_interval: BackgroundImageRotationInterval::Daily,
        }
    }

    #[test]
    fn stable_hash_matches_frontend_fnv1a() {
        assert_eq!(stable_hash(""), 2166136261);
        assert_eq!(stable_hash("a"), 0xe40c292c);
        assert_eq!(stable_hash("files:C:\\img\\a.png:2026-07-30"), {
            let mut hash: u32 = 2166136261;
            for unit in "files:C:\\img\\a.png:2026-07-30".encode_utf16() {
                hash ^= unit as u32;
                hash = hash.wrapping_mul(16777619);
            }
            hash
        });
    }

    #[test]
    fn custom_source_normalization_drops_empty_sources() {
        assert!(normalize_custom_source_struct(files_custom_source(&[])).is_none());
        assert!(normalize_custom_source_struct(BackgroundImageCustomSource {
            kind: BackgroundImageCustomSourceKind::Folder,
            paths: Vec::new(),
            folder_path: "  ".into(),
            rotation_interval: BackgroundImageRotationInterval::Daily,
        })
        .is_none());
        let normalized =
            normalize_custom_source_struct(files_custom_source(&["a.png", " a.png ", "", "b.png"]))
                .unwrap();
        assert_eq!(normalized.paths, vec!["a.png", "b.png"]);
    }

    #[test]
    fn custom_source_wire_normalization_matches_config_shape() {
        let value = json!({
            "kind": "folder",
            "paths": ["ignored.png"],
            "folderPath": " C:\\wallpapers ",
            "rotationInterval": "hourly"
        });
        let source = normalize_custom_source(&value).unwrap();
        assert_eq!(source.kind, BackgroundImageCustomSourceKind::Folder);
        assert!(source.paths.is_empty());
        assert_eq!(source.folder_path, "C:\\wallpapers");
        assert_eq!(
            source.rotation_interval,
            BackgroundImageRotationInterval::Hourly
        );
    }

    #[test]
    fn provider_snapshot_normalization_accepts_legacy_resolved_for_date() {
        let value = json!({
            "providerId": "nasa-epic",
            "imageUrl": "https://epic.gsfc.nasa.gov/a.jpg",
            "resolvedAt": "2026-07-30T00:00:00.000Z",
            "resolvedForDate": "2026-07-30"
        });
        let snapshot =
            normalize_provider_snapshot(Some(&value), BackgroundImageProviderId::NasaEpic).unwrap();
        assert_eq!(snapshot.resolved_for_key, "2026-07-30");
        assert!(
            normalize_provider_snapshot(Some(&value), BackgroundImageProviderId::NasaApodSafe)
                .is_none()
        );
    }

    #[test]
    fn snapshot_freshness_uses_24h_ttl() {
        let mut snapshot = BackgroundImageSnapshot {
            mode: BackgroundImageMode::Daily,
            provider_id: Some(BackgroundImageProviderId::NasaEpic),
            source_kind: None,
            image_url: "https://example.com/a.jpg".into(),
            image_path: None,
            image_count: None,
            title: String::new(),
            author: String::new(),
            license: String::new(),
            source: String::new(),
            resolved_at: (Utc::now() - chrono::Duration::hours(23)).to_rfc3339(),
            resolved_for_key: "2026-07-30".into(),
        };
        assert!(is_snapshot_fresh(Some(&snapshot)));
        snapshot.resolved_at = (Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
        assert!(!is_snapshot_fresh(Some(&snapshot)));
        snapshot.resolved_at = String::new();
        assert!(!is_snapshot_fresh(Some(&snapshot)));
        assert!(!is_snapshot_fresh(None));
    }

    #[test]
    fn rotation_boundary_aligns_to_next_hour_and_day() {
        let now = Local.with_ymd_and_hms(2026, 7, 30, 10, 15, 30).unwrap();
        let hourly = duration_until_next_rotation(BackgroundImageRotationInterval::Hourly, now);
        assert_eq!(hourly, Duration::from_millis((44 * 60 + 32) * 1000));
        let daily = duration_until_next_rotation(BackgroundImageRotationInterval::Daily, now);
        assert_eq!(
            daily,
            Duration::from_millis(((13 * 60 + 44) * 60 + 32) * 1000)
        );
    }

    #[test]
    fn rotation_key_uses_local_date_and_hour() {
        let now = Local.with_ymd_and_hms(2026, 7, 30, 9, 5, 0).unwrap();
        assert_eq!(
            rotation_key(BackgroundImageRotationInterval::Daily, now),
            "2026-07-30"
        );
        assert_eq!(
            rotation_key(BackgroundImageRotationInterval::Hourly, now),
            "2026-07-30T09"
        );
    }

    #[test]
    fn selected_files_assertions_detect_missing_paths() {
        let source = files_custom_source(&["C:\\img\\A.png", "C:\\img\\b.png"]);
        let files = vec!["c:\\img\\a.png".to_string(), "C:\\img\\b.png".to_string()];
        assert!(assert_selected_files_available(&source, &files).is_ok());
        assert!(assert_selected_files_available(&source, &files[..1].to_vec()).is_err());
    }
}
