use std::path::PathBuf;

use vrcx_0_application_core::{Error, Result};
use vrcx_0_integrations::external_api;
use vrcx_0_persistence::config::{self, resolve_config_key, ConfigReadEntry, ConfigWriteEntry};
use vrcx_0_persistence::DatabaseService;

pub fn list_config_values(db: &DatabaseService) -> Result<Vec<ConfigReadEntry>> {
    Ok(config::config_list_values(db)?)
}

pub fn set_config_values(db: &DatabaseService, entries: Vec<ConfigWriteEntry>) -> Result<()> {
    for entry in &entries {
        validate_config_mutation(&entry.key, Some(&entry.value))?;
    }
    config::config_set_values(db, entries)?;
    Ok(())
}

pub fn remove_config_value(db: &DatabaseService, key: String) -> Result<i64> {
    validate_config_mutation(&key, None)?;
    Ok(config::config_remove_value(db, key)?)
}

fn validate_config_mutation(key: &str, value: Option<&str>) -> Result<()> {
    match resolve_config_key(key).as_str() {
        "config:vrcx_savedcredentials" => Err(Error::Custom(
            "savedCredentials must be changed through the dedicated auth service.".into(),
        )),
        "config:vrcx_usergeneratedcontentpath" => validate_ugc_path(value.unwrap_or_default()),
        "config:vrcx_translationapiendpoint" => validate_optional_provider_url(
            value.unwrap_or_default(),
            "translationAPIEndpoint must be an HTTP or HTTPS endpoint.",
        ),
        "config:vrcx_avatarremotedatabaseprovider" => validate_optional_provider_url(
            value.unwrap_or_default(),
            "VRCX_avatarRemoteDatabaseProvider must be an HTTP or HTTPS endpoint.",
        ),
        "config:vrcx_avatarremotedatabaseproviderlist" => {
            validate_provider_list(value.unwrap_or_default())
        }
        _ => Ok(()),
    }
}

fn validate_ugc_path(value: &str) -> Result<()> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(Error::Custom(
            "userGeneratedContentPath must be an absolute folder path.".into(),
        ));
    }
    if path.exists() && !path.is_dir() {
        return Err(Error::Custom(
            "userGeneratedContentPath must point to a folder.".into(),
        ));
    }
    Ok(())
}

fn validate_optional_provider_url(value: &str, message: &str) -> Result<()> {
    let value = value.trim();
    if value.is_empty() || external_api::request_origin(value).is_some() {
        return Ok(());
    }
    Err(Error::Custom(message.into()))
}

fn validate_provider_list(value: &str) -> Result<()> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    let providers: Vec<String> = serde_json::from_str(value).map_err(|error| {
        Error::Custom(format!(
            "VRCX_avatarRemoteDatabaseProviderList must be a JSON string array: {error}"
        ))
    })?;
    for provider in providers {
        validate_optional_provider_url(
            &provider,
            "VRCX_avatarRemoteDatabaseProviderList contains a non-HTTP(S) endpoint.",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-application-config-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_db(name: &str) -> (TestDir, Arc<DatabaseService>) {
        let dir = TestDir::new(name);
        let db = Arc::new(DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap());
        (dir, db)
    }

    fn entry(key: &str, value: &str) -> ConfigWriteEntry {
        ConfigWriteEntry {
            key: key.into(),
            value: value.into(),
        }
    }

    #[test]
    fn accepts_regular_config_and_http_providers() {
        let (_dir, db) = test_db("valid");

        set_config_values(
            &db,
            vec![
                entry("SomeRegularSetting", "anything"),
                entry(
                    "translationAPIEndpoint",
                    "http://localhost:8123/v1/chat/completions",
                ),
                entry(
                    "VRCX_avatarRemoteDatabaseProviderList",
                    r#"["http://127.0.0.1:8123/api","https://10.0.0.5/api"]"#,
                ),
            ],
        )
        .unwrap();
    }

    #[test]
    fn rejects_invalid_provider_and_relative_ugc_writes() {
        let (_dir, db) = test_db("invalid");

        assert!(set_config_values(
            &db,
            vec![entry("translationAPIEndpoint", "ftp://example.com/api")]
        )
        .is_err());
        assert!(set_config_values(
            &db,
            vec![entry("userGeneratedContentPath", "relative/path")]
        )
        .is_err());
    }

    #[test]
    fn dedicated_auth_config_cannot_be_set_or_removed() {
        let (_dir, db) = test_db("auth-owner");

        assert!(set_config_values(&db, vec![entry("savedCredentials", "{}")]).is_err());
        assert!(remove_config_value(&db, "config:vrcx_savedcredentials".into()).is_err());
    }

    #[test]
    fn regular_removal_preserves_the_existing_count_contract() {
        let (_dir, db) = test_db("remove");
        set_config_values(&db, vec![entry("ThemeMode", "dark")]).unwrap();

        assert_eq!(remove_config_value(&db, "ThemeMode".into()).unwrap(), 1);
        assert_eq!(remove_config_value(&db, "ThemeMode".into()).unwrap(), 0);
    }
}
