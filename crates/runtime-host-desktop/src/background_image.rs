use std::fs;
use std::path::{Path, PathBuf};

use vrcx_0_application::{
    BackgroundImageCustomSource, BackgroundImageCustomSourceKind, BackgroundImageFileResolver,
};
use vrcx_0_application_core::Error;

use crate::HostFileAccess;

pub const BACKGROUND_IMAGE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

fn is_background_image_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    BACKGROUND_IMAGE_EXTENSIONS
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
}

fn background_image_files_in_folder(folder: &Path) -> Result<Vec<String>, Error> {
    if !folder.is_dir() {
        return Err(Error::Custom(
            "Background image folder is not available.".into(),
        ));
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(folder).map_err(Error::from)? {
        let entry = entry.map_err(Error::from)?;
        let path = entry.path();
        if is_background_image_file(&path) {
            files.push(path.to_string_lossy().to_string());
        }
    }
    files.sort_by_key(|path| path.to_ascii_lowercase());
    Ok(files)
}

pub fn background_image_files_from_paths(paths: Vec<String>) -> Vec<String> {
    let mut files: Vec<String> = paths
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| is_background_image_file(path))
        .map(|path| path.to_string_lossy().to_string())
        .collect();
    files.sort_by_key(|path| path.to_ascii_lowercase());
    files.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    files
}

pub struct HostBackgroundImageFileResolver {
    host_file_access: HostFileAccess,
}

impl HostBackgroundImageFileResolver {
    pub fn new(host_file_access: HostFileAccess) -> Self {
        Self { host_file_access }
    }
}

impl BackgroundImageFileResolver for HostBackgroundImageFileResolver {
    fn resolve_files(
        &self,
        source: &BackgroundImageCustomSource,
    ) -> Result<Vec<String>, Error> {
        let files = match source.kind {
            BackgroundImageCustomSourceKind::Folder => {
                let folder = PathBuf::from(&source.folder_path);
                self.host_file_access.register_path(&folder);
                background_image_files_in_folder(&folder)?
            }
            BackgroundImageCustomSourceKind::Files => {
                background_image_files_from_paths(source.paths.clone())
            }
        };

        for file in &files {
            self.host_file_access.register_path(file);
        }
        Ok(files)
    }
}
