#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    #[serde(default)]
    pub version: i32,
    pub author: AuthorDetail,
    pub world: WorldDetail,
    pub players: Vec<PlayerDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<[f32; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotLibraryScanStatus {
    pub running: bool,
    pub scanned: usize,
    pub indexed: usize,
    pub changed: usize,
    pub skipped: usize,
    pub deleted: usize,
    pub error: Option<String>,
    pub last_scan_at: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotFolderInfo {
    pub path: String,
    pub parent_path: Option<String>,
    pub name: String,
    pub image_count: usize,
    pub total_image_count: usize,
    pub latest_modified_at: Option<i64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotFolderTree {
    pub root_path: String,
    pub folders: Vec<ScreenshotFolderInfo>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotLibraryImage {
    pub path: String,
    pub folder_path: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub modified_at: i64,
    pub created_at: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub world_id: Option<String>,
    pub world_name: Option<String>,
    pub captured_at: Option<String>,
    pub metadata: Option<ScreenshotMetadata>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AuthorDetail {
    #[serde(default)]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldDetail {
    #[serde(default)]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub instance_id: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetail {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<[f32; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenshotSearchType {
    Username = 0,
    UserId = 1,
    WorldName = 2,
    WorldId = 3,
}

impl ScreenshotSearchType {
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::UserId,
            2 => Self::WorldName,
            3 => Self::WorldId,
            _ => Self::Username,
        }
    }
}

impl ScreenshotMetadata {
    pub fn just_error(source_file: &str, error: &str) -> Self {
        Self {
            source_file: Some(source_file.into()),
            error: Some(error.into()),
            ..Default::default()
        }
    }

    pub fn contains_player_id(&self, id: &str) -> bool {
        self.players.iter().any(|p| p.id == id)
    }

    pub fn contains_player_name(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.players
            .iter()
            .any(|p| p.display_name.to_lowercase().contains(&lower))
    }
}

pub fn parse_vrc_image(xml_string: &str) -> ScreenshotMetadata {
    let idx = match xml_string.find("<x:xmpmeta") {
        Some(i) => i,
        None => return ScreenshotMetadata::default(),
    };
    let xml = &xml_string[idx..];

    let mut creator_tool: Option<String> = None;
    let mut author_name: Option<String> = None;
    let mut author_id: Option<String> = None;
    let mut date_time: Option<String> = None;
    let mut note: Option<String> = None;
    let mut world_id: Option<String> = None;
    let mut world_display_name: Option<String> = None;

    use quick_xml::escape::unescape;
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                current_tag = name;
            }
            Ok(Event::Text(ref e)) => {
                let text = e
                    .decode()
                    .ok()
                    .and_then(|text| unescape(&text).ok().map(|text| text.into_owned()))
                    .unwrap_or_default();
                if text.trim().is_empty() {
                    continue;
                }
                match current_tag.as_str() {
                    "CreatorTool" => creator_tool = Some(text),
                    "Author" => author_name = Some(text),
                    "DateTime" => date_time = Some(text),
                    "li" if note.is_none() => {
                        note = Some(text);
                    }
                    "WorldID" | "World" if world_id.is_none() => {
                        world_id = Some(text);
                    }
                    "WorldDisplayName" => world_display_name = Some(text),
                    "AuthorID" => author_id = Some(text),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if author_id.is_none() {
        author_id = author_name.take();
    }

    ScreenshotMetadata {
        application: creator_tool,
        version: 1,
        author: AuthorDetail {
            id: author_id.unwrap_or_default(),
            display_name: author_name,
        },
        world: WorldDetail {
            id: world_id.clone().unwrap_or_default(),
            name: world_display_name,
            instance_id: world_id.unwrap_or_default(),
        },
        timestamp: date_time,
        note,
        ..Default::default()
    }
}

pub fn parse_lfs_picture(metadata_string: &str) -> ScreenshotMetadata {
    let mut metadata = ScreenshotMetadata::default();
    let mut parts: Vec<&str> = metadata_string.split('|').collect();

    if parts.len() > 1 && parts[1] == "cvr" {
        parts.remove(0);
    }

    if parts.len() < 2 {
        return metadata;
    }

    let application = parts[0];
    let version: i32 = parts[1].parse().unwrap_or(0);
    metadata.application = Some(application.into());
    metadata.version = version;

    let is_cvr = application == "cvr";

    if application == "screenshotmanager" {
        if parts.len() >= 4 {
            let mut author_parts = parts[2]
                .strip_prefix("author:")
                .unwrap_or(parts[2])
                .splitn(2, ',');
            if let (Some(id), Some(display_name)) = (author_parts.next(), author_parts.next()) {
                metadata.author.id = id.into();
                metadata.author.display_name = Some(display_name.into());
            }
            let mut world_parts = parts[3].splitn(3, ',');
            if let (Some(id), Some(instance_id), Some(name)) =
                (world_parts.next(), world_parts.next(), world_parts.next())
            {
                metadata.world.id = id.into();
                metadata.world.name = Some(name.into());
                metadata.world.instance_id = format!("{id}:{instance_id}");
            }
        }
        return metadata;
    }

    for part in parts.iter().skip(2) {
        let split: Vec<&str> = part.splitn(2, ':').collect();
        if split.len() < 2 || split[1].is_empty() {
            continue;
        }
        let key = split[0];
        let value = split[1];

        match key {
            "author" => {
                let Some((id, display_name)) = value.split_once(',') else {
                    continue;
                };
                metadata.author.id = if is_cvr { String::new() } else { id.into() };
                metadata.author.display_name = Some(if is_cvr {
                    format!("{display_name} ({id})")
                } else {
                    display_name.into()
                });
            }
            "world" => {
                if is_cvr || version == 1 {
                    metadata.world.id = String::new();
                    metadata.world.instance_id = String::new();
                    metadata.world.name = Some(if is_cvr {
                        let mut world_parts = value.splitn(3, ',');
                        match (world_parts.next(), world_parts.next(), world_parts.next()) {
                            (Some(id), Some(_), Some(name)) => format!("{name} ({id})"),
                            _ => value.into(),
                        }
                    } else {
                        value.into()
                    });
                } else {
                    let mut world_parts = value.splitn(3, ',');
                    if let (Some(id), Some(instance_id), Some(name)) =
                        (world_parts.next(), world_parts.next(), world_parts.next())
                    {
                        metadata.world.id = id.into();
                        metadata.world.instance_id = format!("{id}:{instance_id}");
                        metadata.world.name = Some(name.into());
                    }
                }
            }
            "pos" => {
                let coordinates = value.splitn(3, ',').collect::<Vec<_>>();
                if coordinates.len() >= 3 {
                    let x: f32 = coordinates[0].parse().unwrap_or(0.0);
                    let y: f32 = coordinates[1].parse().unwrap_or(0.0);
                    let z: f32 = coordinates[2].parse().unwrap_or(0.0);
                    metadata.pos = Some([x, y, z]);
                }
            }
            "players" => {
                let players_str = value.split(';');
                for player in players_str {
                    let pp: Vec<&str> = player.splitn(5, ',').collect();
                    if pp.len() >= 5 {
                        let x: f32 = pp[1].parse().unwrap_or(0.0);
                        let y: f32 = pp[2].parse().unwrap_or(0.0);
                        let z: f32 = pp[3].parse().unwrap_or(0.0);
                        metadata.players.push(PlayerDetail {
                            id: if is_cvr { String::new() } else { pp[0].into() },
                            display_name: if is_cvr {
                                format!("{} ({})", pp[4], pp[0])
                            } else {
                                pp[4].into()
                            },
                            pos: Some([x, y, z]),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    metadata
}

#[cfg(test)]
mod tests;
