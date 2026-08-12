use vrcx_0_persistence::config::resolve_config_key;
use vrcx_0_persistence::favorites;
use vrcx_0_persistence::favorites::FavoriteRow;
use vrcx_0_persistence::DatabaseService;

use crate::{AuthenticatedMutationContext, Error, Result};
use vrcx_0_application_core::{
    read_config_string_array, write_config_string_array, FavoriteChange, FavoriteChangeScope,
    FavoriteEntityKind, FavoritesChangedPayload, RuntimeEventBus,
};

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalFavoriteGroupWrite {
    pub config_key: String,
    pub group_names: Vec<String>,
    pub affected: i64,
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalFavoriteSnapshot {
    pub favorites: Vec<FavoriteRow>,
    pub group_names: Vec<String>,
}

pub(super) struct LocalFavoriteMutationDeps<'a> {
    pub db: &'a DatabaseService,
    pub event_bus: &'a RuntimeEventBus,
    pub mutation: AuthenticatedMutationContext<'a>,
}

fn notify_local_favorite_change(
    deps: &LocalFavoriteMutationDeps<'_>,
    kind: FavoriteEntityKind,
    change: FavoriteChange,
) {
    let payload = if kind == FavoriteEntityKind::World {
        FavoritesChangedPayload::invalidated(
            deps.mutation.scope(),
            FavoriteChangeScope::World,
            true,
            false,
        )
    } else {
        FavoritesChangedPayload::from_changes(
            deps.mutation.scope(),
            kind.into(),
            true,
            false,
            vec![change],
        )
    };
    deps.event_bus.emit_favorites_changed(payload);
}

pub(super) fn add_local_favorite_scoped(
    deps: &LocalFavoriteMutationDeps<'_>,
    kind: FavoriteEntityKind,
    entity_id: String,
    group_name: String,
) -> Result<i64> {
    deps.mutation.ensure_current()?;
    let affected = add_local_favorite(
        deps.db,
        &deps.mutation.scope().current_user_id,
        kind,
        entity_id.clone(),
        group_name.clone(),
    )?;
    deps.mutation.ensure_current()?;
    notify_local_favorite_change(
        deps,
        kind,
        FavoriteChange::LocalAdded {
            kind,
            entity_id,
            group_name,
        },
    );
    Ok(affected)
}

pub(super) fn remove_local_favorite_scoped(
    deps: &LocalFavoriteMutationDeps<'_>,
    kind: FavoriteEntityKind,
    entity_id: String,
    group_name: String,
) -> Result<i64> {
    deps.mutation.ensure_current()?;
    let affected = remove_local_favorite(
        deps.db,
        &deps.mutation.scope().current_user_id,
        kind,
        entity_id.clone(),
        group_name.clone(),
    )?;
    deps.mutation.ensure_current()?;
    notify_local_favorite_change(
        deps,
        kind,
        FavoriteChange::LocalRemoved {
            kind,
            entity_id,
            group_name,
        },
    );
    Ok(affected)
}

pub fn list_local_favorites(
    db: &DatabaseService,
    owner_user_id: &str,
    kind: FavoriteEntityKind,
) -> Result<Vec<FavoriteRow>> {
    favorites::favorite_list(db, Some(owner_user_id), kind).map_err(Error::from)
}

pub fn get_local_favorite_snapshot(
    db: &DatabaseService,
    owner_user_id: &str,
    kind: FavoriteEntityKind,
) -> Result<LocalFavoriteSnapshot> {
    let favorites = list_local_favorites(db, owner_user_id, kind)?;
    let mut group_names = read_config_string_array(db, local_group_config_key(kind))?;
    if kind == FavoriteEntityKind::Friend && !owner_user_id.trim().is_empty() {
        group_names.extend(read_config_string_array(
            db,
            &writable_group_config_key(kind, owner_user_id),
        )?);
        group_names.sort();
        group_names.dedup();
    }
    Ok(LocalFavoriteSnapshot {
        favorites,
        group_names,
    })
}

pub(crate) fn add_local_favorite(
    db: &DatabaseService,
    owner_user_id: &str,
    kind: FavoriteEntityKind,
    entity_id: String,
    group_name: String,
) -> Result<i64> {
    favorites::favorite_add(db, Some(owner_user_id), kind, entity_id, group_name)
        .map_err(Error::from)
}

pub(crate) fn remove_local_favorite(
    db: &DatabaseService,
    owner_user_id: &str,
    kind: FavoriteEntityKind,
    entity_id: String,
    group_name: String,
) -> Result<i64> {
    favorites::favorite_remove(db, Some(owner_user_id), kind, entity_id, group_name)
        .map_err(Error::from)
}

pub(super) const fn local_group_config_key(kind: FavoriteEntityKind) -> &'static str {
    match kind {
        FavoriteEntityKind::Friend => "localFavoriteFriendGroups",
        FavoriteEntityKind::Avatar => "localFavoriteAvatarGroups",
        FavoriteEntityKind::World => "localFavoriteWorldGroups",
    }
}

fn add_group_value(groups: &mut Vec<String>, group_name: &str) {
    if groups.iter().any(|value| value == group_name) {
        return;
    }
    groups.push(group_name.to_string());
    groups.sort();
    groups.dedup();
}

pub(crate) fn create_local_favorite_group(
    db: &DatabaseService,
    owner_user_id: &str,
    kind: FavoriteEntityKind,
    group_name: String,
) -> Result<LocalFavoriteGroupWrite> {
    let key = writable_group_config_key(kind, owner_user_id);
    let mut groups = read_config_string_array(db, &key)?;
    add_group_value(&mut groups, &group_name);
    write_config_string_array(db, &key, &groups)?;
    Ok(LocalFavoriteGroupWrite {
        config_key: resolve_config_key(&key),
        group_names: groups,
        affected: 0,
    })
}

pub(super) fn create_local_favorite_group_scoped(
    deps: &LocalFavoriteMutationDeps<'_>,
    kind: FavoriteEntityKind,
    group_name: String,
) -> Result<LocalFavoriteGroupWrite> {
    deps.mutation.ensure_current()?;
    let write = create_local_favorite_group(
        deps.db,
        &deps.mutation.scope().current_user_id,
        kind,
        group_name.clone(),
    )?;
    deps.mutation.ensure_current()?;
    notify_local_favorite_change(
        deps,
        kind,
        FavoriteChange::LocalGroupCreated { kind, group_name },
    );
    Ok(write)
}

pub(crate) fn rename_local_favorite_group(
    db: &DatabaseService,
    owner_user_id: &str,
    kind: FavoriteEntityKind,
    group_name: String,
    new_group_name: String,
) -> Result<LocalFavoriteGroupWrite> {
    let key = group_config_realm_key(db, kind, owner_user_id, &group_name)?;
    let mut groups = read_config_string_array(db, &key)?
        .into_iter()
        .filter(|value| value != &group_name)
        .collect::<Vec<_>>();
    add_group_value(&mut groups, &new_group_name);
    let affected = favorites::favorite_group_rename_with_config(
        db,
        Some(owner_user_id),
        kind,
        &key,
        &group_name,
        &new_group_name,
        &groups,
    )
    .map_err(Error::from)?;
    Ok(LocalFavoriteGroupWrite {
        config_key: resolve_config_key(&key),
        group_names: groups,
        affected,
    })
}

pub(super) fn rename_local_favorite_group_scoped(
    deps: &LocalFavoriteMutationDeps<'_>,
    kind: FavoriteEntityKind,
    group_name: String,
    new_group_name: String,
) -> Result<LocalFavoriteGroupWrite> {
    deps.mutation.ensure_current()?;
    let write = rename_local_favorite_group(
        deps.db,
        &deps.mutation.scope().current_user_id,
        kind,
        group_name.clone(),
        new_group_name.clone(),
    )?;
    deps.mutation.ensure_current()?;
    notify_local_favorite_change(
        deps,
        kind,
        FavoriteChange::LocalGroupRenamed {
            kind,
            group_name,
            new_group_name,
        },
    );
    Ok(write)
}

pub(crate) fn delete_local_favorite_group(
    db: &DatabaseService,
    owner_user_id: &str,
    kind: FavoriteEntityKind,
    group_name: String,
) -> Result<LocalFavoriteGroupWrite> {
    let key = group_config_realm_key(db, kind, owner_user_id, &group_name)?;
    let groups = read_config_string_array(db, &key)?
        .into_iter()
        .filter(|value| value != &group_name)
        .collect::<Vec<_>>();
    let affected = favorites::favorite_group_delete_with_config(
        db,
        Some(owner_user_id),
        kind,
        &key,
        &group_name,
        &groups,
    )
    .map_err(Error::from)?;
    Ok(LocalFavoriteGroupWrite {
        config_key: resolve_config_key(&key),
        group_names: groups,
        affected,
    })
}

pub(super) fn delete_local_favorite_group_scoped(
    deps: &LocalFavoriteMutationDeps<'_>,
    kind: FavoriteEntityKind,
    group_name: String,
) -> Result<LocalFavoriteGroupWrite> {
    deps.mutation.ensure_current()?;
    let write = delete_local_favorite_group(
        deps.db,
        &deps.mutation.scope().current_user_id,
        kind,
        group_name.clone(),
    )?;
    deps.mutation.ensure_current()?;
    notify_local_favorite_change(
        deps,
        kind,
        FavoriteChange::LocalGroupDeleted { kind, group_name },
    );
    Ok(write)
}

fn writable_group_config_key(kind: FavoriteEntityKind, owner_user_id: &str) -> String {
    let base_key = local_group_config_key(kind);
    if kind == FavoriteEntityKind::Friend && !owner_user_id.trim().is_empty() {
        format!("{base_key}:{}", owner_user_id.trim())
    } else {
        base_key.to_string()
    }
}

fn group_config_realm_key(
    db: &DatabaseService,
    kind: FavoriteEntityKind,
    owner_user_id: &str,
    group_name: &str,
) -> Result<String> {
    let account_key = writable_group_config_key(kind, owner_user_id);
    if kind != FavoriteEntityKind::Friend
        || read_config_string_array(db, &account_key)?
            .iter()
            .any(|value| value == group_name)
    {
        Ok(account_key)
    } else {
        Ok(local_group_config_key(kind).to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vrcx_0_application_core::{read_config_string_array, write_config_string_array};
    use vrcx_0_persistence::favorites::{favorite_add, favorite_list};

    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-local-favorites-{name}-{}-{nonce}",
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

    #[test]
    fn friend_group_writes_use_account_or_shared_realm() {
        let dir = TestDir::new("group-realms");
        let db = DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap();

        create_local_favorite_group(&db, "usr_a", FavoriteEntityKind::Friend, "account".into())
            .unwrap();
        create_local_favorite_group(&db, "", FavoriteEntityKind::Friend, "legacy".into()).unwrap();
        assert_eq!(
            read_config_string_array(&db, "localFavoriteFriendGroups:usr_a").unwrap(),
            vec!["account"]
        );
        assert_eq!(
            read_config_string_array(&db, "localFavoriteFriendGroups").unwrap(),
            vec!["legacy"]
        );

        favorite_add(
            &db,
            Some("usr_a"),
            FavoriteEntityKind::Friend,
            "usr_account_friend".into(),
            "account".into(),
        )
        .unwrap();
        favorite_add(
            &db,
            None,
            FavoriteEntityKind::Friend,
            "usr_legacy_friend".into(),
            "legacy".into(),
        )
        .unwrap();

        rename_local_favorite_group(
            &db,
            "usr_a",
            FavoriteEntityKind::Friend,
            "account".into(),
            "renamed".into(),
        )
        .unwrap();
        delete_local_favorite_group(&db, "usr_a", FavoriteEntityKind::Friend, "legacy".into())
            .unwrap();

        let groups = favorite_list(&db, Some("usr_a"), FavoriteEntityKind::Friend)
            .unwrap()
            .into_iter()
            .map(|row| row.group_name)
            .collect::<Vec<_>>();
        assert_eq!(groups, vec!["renamed"]);
        assert_eq!(
            read_config_string_array(&db, "localFavoriteFriendGroups:usr_a").unwrap(),
            vec!["renamed"]
        );
        assert!(read_config_string_array(&db, "localFavoriteFriendGroups")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn local_world_snapshot_reads_favorites_and_explicit_groups_together() {
        let dir = TestDir::new("world-snapshot");
        let db = DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap();
        write_config_string_array(
            &db,
            "localFavoriteWorldGroups",
            &["Empty".into(), "Worlds".into()],
        )
        .unwrap();
        favorite_add(
            &db,
            Some("usr_a"),
            FavoriteEntityKind::World,
            "wrld_1".into(),
            "Worlds".into(),
        )
        .unwrap();

        let snapshot =
            get_local_favorite_snapshot(&db, "usr_a", FavoriteEntityKind::World).unwrap();

        assert_eq!(snapshot.group_names, vec!["Empty", "Worlds"]);
        assert_eq!(snapshot.favorites.len(), 1);
        assert_eq!(snapshot.favorites[0].world_id.as_deref(), Some("wrld_1"));
        assert_eq!(snapshot.favorites[0].group_name, "Worlds");
    }

    #[test]
    fn account_group_rename_does_not_rewrite_shared_rows_with_same_name() {
        let dir = TestDir::new("same-name-realms");
        let db = DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap();
        write_config_string_array(&db, "localFavoriteFriendGroups", &["same".into()]).unwrap();
        write_config_string_array(&db, "localFavoriteFriendGroups:usr_a", &["same".into()])
            .unwrap();
        favorite_add(
            &db,
            None,
            FavoriteEntityKind::Friend,
            "usr_shared".into(),
            "same".into(),
        )
        .unwrap();
        favorite_add(
            &db,
            Some("usr_a"),
            FavoriteEntityKind::Friend,
            "usr_account".into(),
            "same".into(),
        )
        .unwrap();

        rename_local_favorite_group(
            &db,
            "usr_a",
            FavoriteEntityKind::Friend,
            "same".into(),
            "account-only".into(),
        )
        .unwrap();

        let mut rows = favorite_list(&db, Some("usr_a"), FavoriteEntityKind::Friend)
            .unwrap()
            .into_iter()
            .map(|row| (row.user_id.unwrap_or_default(), row.group_name))
            .collect::<Vec<_>>();
        rows.sort();
        assert_eq!(
            rows,
            vec![
                ("usr_account".into(), "account-only".into()),
                ("usr_shared".into(), "same".into()),
            ]
        );
    }
}
