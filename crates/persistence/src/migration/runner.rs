use serde_json::Value;

use crate::database::DatabaseWriteTransaction;
use crate::migration::types::{
    Migration, MigrationTx, PendingMigration, Preview, PreviewStatus, ProgressSink, Report, Step,
    Target,
};
use crate::{DatabaseService, Error};

impl MigrationTx for DatabaseWriteTransaction<'_> {
    fn execute_non_query(&self, sql: &str) -> Result<i64, Error> {
        DatabaseWriteTransaction::execute_non_query(self, sql, &Default::default())
    }

    fn query(&self, sql: &str) -> Result<Vec<Vec<Value>>, Error> {
        DatabaseWriteTransaction::execute(self, sql, &Default::default())
    }
}

pub fn migration_version(db: &DatabaseService) -> Result<i64, Error> {
    Ok(db
        .execute("SELECT * FROM pragma_user_version", &Default::default())?
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.as_i64())
        .unwrap_or(0))
}

fn pending_migrations(
    migrations: &[Migration],
    current_version: i64,
) -> impl Iterator<Item = &Migration> {
    migrations
        .iter()
        .filter(move |migration| migration.version > current_version)
}

fn target_version(migrations: &[Migration], current_version: i64) -> i64 {
    migrations
        .last()
        .map(|migration| migration.version)
        .unwrap_or(current_version)
}

pub fn preview(db: &DatabaseService, migrations: &[Migration]) -> Result<Preview, Error> {
    validate(migrations)?;
    let current_version = migration_version(db)?;
    let target_version = target_version(migrations, current_version);
    let pending: Vec<PendingMigration> = pending_migrations(migrations, current_version)
        .map(|migration| PendingMigration {
            version: migration.version,
            label: migration.label,
        })
        .collect();
    let status = if !pending.is_empty() {
        PreviewStatus::Pending
    } else if current_version > target_version {
        PreviewStatus::NewerSchema
    } else {
        PreviewStatus::Current
    };
    Ok(Preview {
        status,
        current_version,
        target_version,
        pending,
    })
}

pub fn run(
    db: &DatabaseService,
    migrations: &[Migration],
    sink: &dyn ProgressSink,
) -> Result<Report, Error> {
    validate(migrations)?;
    let from_version = migration_version(db)?;
    let target_version = target_version(migrations, from_version);
    if from_version > target_version {
        return Err(Error::Database(format!(
            "Database migration version {from_version} is newer than this build supports ({target_version})."
        )));
    }
    let pending: Vec<&Migration> = pending_migrations(migrations, from_version).collect();

    let total = pending.len();
    let mut applied = Vec::new();
    for (index, migration) in pending.into_iter().enumerate() {
        sink.on_start(migration.version, migration.label, index, total);
        apply(db, migration)?;
        applied.push(migration.version);
        sink.on_finish(migration.version);
    }
    Ok(Report {
        from_version,
        to_version: applied.last().copied().unwrap_or(from_version),
        applied,
    })
}

fn apply(db: &DatabaseService, migration: &Migration) -> Result<(), Error> {
    db.write_transaction(|tx| {
        for step in &migration.steps {
            apply_step(tx, step)?;
        }
        if let Some(verify) = &migration.verify {
            verify.run(tx, &Target::global())?;
        }
        MigrationTx::execute_non_query(
            tx,
            &format!("PRAGMA user_version = {}", migration.version),
        )?;
        Ok(())
    })
}

fn apply_step(tx: &dyn MigrationTx, step: &Step) -> Result<(), Error> {
    match step {
        Step::Sql(sql) => {
            tx.execute_non_query(sql)?;
        }
        Step::Custom(run) => {
            run.run(tx, &Target::global())?;
        }
        Step::PerUser { suffix, build } => {
            for table in per_user_tables(tx, suffix)? {
                for sql in build(&table) {
                    tx.execute_non_query(&sql)?;
                }
            }
        }
        Step::PerUserCustom { suffix, run } => {
            for table in per_user_tables(tx, suffix)? {
                run.run(
                    tx,
                    &Target {
                        table: Some(&table),
                        user_prefix: user_prefix_of(&table, suffix),
                    },
                )?;
            }
        }
    }
    Ok(())
}

fn per_user_tables(tx: &dyn MigrationTx, suffix: &str) -> Result<Vec<String>, Error> {
    if suffix.is_empty()
        || !suffix
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '_')
    {
        return Err(Error::Database(format!(
            "Invalid per-user table suffix: {suffix}"
        )));
    }
    let rows = tx.query(&format!(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name GLOB 'usr*_{suffix}' ORDER BY name"
    ))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.into_iter().next())
        .filter_map(|value| value.as_str().map(str::to_owned))
        .filter(|table| user_prefix_of(table, suffix).is_some())
        .collect())
}

fn user_prefix_of<'a>(table: &'a str, suffix: &str) -> Option<&'a str> {
    let prefix = table.strip_suffix(suffix)?.strip_suffix('_')?;
    let identifier = prefix.strip_prefix("usr")?;
    (!identifier.is_empty()
        && identifier
            .chars()
            .all(|value| value.is_ascii_alphanumeric()))
    .then_some(prefix)
}

fn validate(migrations: &[Migration]) -> Result<(), Error> {
    if let Some(first) = migrations.first() {
        if first.version < 1 {
            return Err(Error::Database(format!(
                "Migration versions must start at 1, found {}",
                first.version
            )));
        }
    }
    for pair in migrations.windows(2) {
        if pair[1].version <= pair[0].version {
            return Err(Error::Database(format!(
                "Migration list must be strictly ascending: {} follows {}",
                pair[1].version, pair[0].version
            )));
        }
    }
    Ok(())
}
