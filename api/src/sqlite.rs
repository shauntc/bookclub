use anyhow::Result;
use serde::Deserialize;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub url: String,
}

pub async fn create_pool(settings: &Settings) -> Result<SqlitePool> {
    match Sqlite::database_exists(&settings.url).await? {
        true => tracing::info!("Database already exists"),
        false => Sqlite::create_database(&settings.url).await?,
    }
    let pool = SqlitePool::connect(&settings.url).await?;

    sqlx::migrate!("db/migrations").run(&pool).await?;

    Ok(pool)
}
