use std::{fs, path::Path, str::FromStr};

use anyhow::Result;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

static MIGRATOR: Migrator = sqlx::migrate!("./db/migrations");

#[tokio::main]
async fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=db/migrations");
    println!("cargo:rerun-if-changed=config");

    let current_dir = std::env::current_dir()?;

    setup_db(&current_dir).await?;

    copy_config_to_output()?;

    Ok(())
}

async fn setup_db(current_dir: &Path) -> Result<()> {
    let db_path = current_dir.join("db/bookclub.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());
    let db_url = &db_url;

    println!("cargo:warning=Ensuring database exists at {}", &db_url);

    let connect_options = SqliteConnectOptions::from_str(db_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(connect_options)
        .await?;

    println!("cargo:warning=Running migrations on {}...", db_url);
    MIGRATOR.run(&pool).await?;
    println!("cargo:warning=Build script: Migrations complete.");

    // DATABASE_URL is used by sqlx to connect to the database
    println!("cargo::rustc-env=DATABASE_URL={}", db_url);

    // SQLITE_URL is read into the config
    println!("cargo::rustc-env=SQLITE_URL={}", db_url);

    Ok(())
}

fn copy_config_to_output() -> Result<()> {
    for file in fs::read_dir("config")? {
        let file = file?;
        let path = file.path();
        let config_name = file
            .file_name()
            .to_string_lossy()
            .rsplit_once('.')
            .map(|(name, _)| name)
            .unwrap_or_default()
            .to_string()
            .to_uppercase();

        let contents = fs::read_to_string(path)?;
        println!("cargo:rustc-env=CONFIG_{}={}", config_name, contents);
        println!("cargo:warning=Set CONFIG_{} with contents", config_name);
    }

    Ok(())
}
