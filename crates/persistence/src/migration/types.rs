use serde_json::Value;

use crate::Error;

pub trait MigrationTx {
    fn execute_non_query(&self, sql: &str) -> Result<i64, Error>;
    fn query(&self, sql: &str) -> Result<Vec<Vec<Value>>, Error>;
}

pub struct Target<'a> {
    pub table: Option<&'a str>,
    pub user_prefix: Option<&'a str>,
}

impl Target<'_> {
    pub fn global() -> Self {
        Self {
            table: None,
            user_prefix: None,
        }
    }

    pub fn require_table(&self) -> Result<&str, Error> {
        self.table
            .ok_or_else(|| Error::Database("Migration step requires a per-user table".into()))
    }
}

pub trait StepFn: Send + Sync {
    fn run(&self, tx: &dyn MigrationTx, target: &Target<'_>) -> Result<(), Error>;
}

impl<F> StepFn for F
where
    F: Fn(&dyn MigrationTx, &Target<'_>) -> Result<(), Error> + Send + Sync,
{
    fn run(&self, tx: &dyn MigrationTx, target: &Target<'_>) -> Result<(), Error> {
        self(tx, target)
    }
}

pub type PerUserBuild = Box<dyn Fn(&str) -> Vec<String> + Send + Sync>;

pub enum Step {
    Sql(String),
    PerUser {
        suffix: &'static str,
        build: PerUserBuild,
    },
    Custom(Box<dyn StepFn>),
    PerUserCustom {
        suffix: &'static str,
        run: Box<dyn StepFn>,
    },
}

impl Step {
    pub fn ddl<S: sea_query::SchemaStatementBuilder>(statement: S) -> Self {
        Self::Sql(statement.to_string(sea_query::SqliteQueryBuilder))
    }

    pub fn dml<S: sea_query::QueryStatementWriter>(statement: S) -> Self {
        Self::Sql(statement.to_string(sea_query::SqliteQueryBuilder))
    }

    pub fn raw(sql: impl Into<String>) -> Self {
        Self::Sql(sql.into())
    }

    pub fn per_user<F>(suffix: &'static str, build: F) -> Self
    where
        F: Fn(&str) -> Vec<String> + Send + Sync + 'static,
    {
        Self::PerUser {
            suffix,
            build: Box::new(build),
        }
    }

    pub fn custom<F>(run: F) -> Self
    where
        F: Fn(&dyn MigrationTx, &Target<'_>) -> Result<(), Error> + Send + Sync + 'static,
    {
        Self::Custom(Box::new(run))
    }

    pub fn per_user_custom<F>(suffix: &'static str, run: F) -> Self
    where
        F: Fn(&dyn MigrationTx, &Target<'_>) -> Result<(), Error> + Send + Sync + 'static,
    {
        Self::PerUserCustom {
            suffix,
            run: Box::new(run),
        }
    }
}

pub struct Migration {
    pub version: i64,
    pub label: &'static str,
    pub steps: Vec<Step>,
    pub verify: Option<Box<dyn StepFn>>,
}

impl Migration {
    pub fn new(version: i64, label: &'static str) -> Self {
        Self {
            version,
            label,
            steps: Vec::new(),
            verify: None,
        }
    }

    pub fn step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    pub fn verify<F>(mut self, verify: F) -> Self
    where
        F: Fn(&dyn MigrationTx, &Target<'_>) -> Result<(), Error> + Send + Sync + 'static,
    {
        self.verify = Some(Box::new(verify));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewStatus {
    Current,
    Pending,
    NewerSchema,
}

#[derive(Clone, Debug)]
pub struct PendingMigration {
    pub version: i64,
    pub label: &'static str,
}

#[derive(Clone, Debug)]
pub struct Preview {
    pub status: PreviewStatus,
    pub current_version: i64,
    pub target_version: i64,
    pub pending: Vec<PendingMigration>,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub from_version: i64,
    pub to_version: i64,
    pub applied: Vec<i64>,
}

pub trait ProgressSink: Sync {
    fn on_start(&self, _version: i64, _label: &str, _index: usize, _total: usize) {}
    fn on_finish(&self, _version: i64) {}
}

pub struct NoopProgress;

impl ProgressSink for NoopProgress {}
