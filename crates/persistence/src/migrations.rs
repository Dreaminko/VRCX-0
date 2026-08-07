use crate::migration::{Migration, Step};

pub fn migrations() -> Vec<Migration> {
    vec![v1_drop_redundant_gamelog_location_index()]
}

fn v1_drop_redundant_gamelog_location_index() -> Migration {
    Migration::new(1, "drop redundant gamelog location index").step(Step::ddl(
        sea_query::Index::drop()
            .name("idx_gamelog_jl_location")
            .if_exists()
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::{migration_version, run, NoopProgress};
    use crate::DatabaseService;

    struct TestDir {
        path: std::path::PathBuf,
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn test_db(name: &str) -> (TestDir, DatabaseService) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vrcx0-migrations-{name}-{nonce}"));
        std::fs::create_dir_all(&path).unwrap();
        let db = DatabaseService::new(&path.join("test.sqlite3")).unwrap();
        (TestDir { path }, db)
    }

    fn index_exists(db: &DatabaseService, name: &str) -> bool {
        db.execute(
            &format!("SELECT name FROM sqlite_master WHERE type='index' AND name='{name}'"),
            &Default::default(),
        )
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
    }

    #[test]
    fn v1_drops_the_redundant_index_and_keeps_the_covering_one() {
        let (_dir, db) = test_db("v1");
        db.execute_non_query(
            "CREATE TABLE gamelog_join_leave (id INTEGER PRIMARY KEY, location TEXT)",
            &Default::default(),
        )
        .unwrap();
        for sql in [
            "CREATE INDEX idx_gamelog_jl_location ON gamelog_join_leave (location)",
            "CREATE INDEX idx_gamelog_jl_location_id ON gamelog_join_leave (location, id)",
        ] {
            db.execute_non_query(sql, &Default::default()).unwrap();
        }

        run(&db, &migrations(), &NoopProgress).unwrap();

        assert!(!index_exists(&db, "idx_gamelog_jl_location"));
        assert!(index_exists(&db, "idx_gamelog_jl_location_id"));
    }

    #[test]
    fn v1_applies_when_the_index_was_never_created() {
        let (_dir, db) = test_db("v1-absent");

        let report = run(&db, &migrations(), &NoopProgress).unwrap();

        assert_eq!(report.applied, vec![1]);
        assert_eq!(migration_version(&db).unwrap(), 1);
    }

    #[test]
    fn list_ascends_strictly_from_one() {
        let versions: Vec<i64> = migrations().iter().map(|entry| entry.version).collect();
        assert_eq!(versions.first(), Some(&1));
        assert!(versions.windows(2).all(|pair| pair[1] > pair[0]));
    }
}
