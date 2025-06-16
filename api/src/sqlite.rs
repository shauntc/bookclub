use anyhow::Result;
use axum::extract::FromRef;
use serde::Deserialize;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct Database(SqlitePool);

impl Database {
    pub async fn new(settings: &Settings) -> Result<Database> {
        match Sqlite::database_exists(&settings.url).await? {
            true => tracing::info!("Database already exists"),
            false => Sqlite::create_database(&settings.url).await?,
        }
        let pool = SqlitePool::connect(&settings.url).await?;

        sqlx::migrate!("db/migrations").run(&pool).await?;

        Ok(Database(pool))
    }
}

impl FromRef<AppState> for Database {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl AsRef<SqlitePool> for Database {
    fn as_ref(&self) -> &SqlitePool {
        &self.0
    }
}

impl AsMut<SqlitePool> for Database {
    fn as_mut(&mut self) -> &mut SqlitePool {
        &mut self.0
    }
}
