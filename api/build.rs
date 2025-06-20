use std::{fs, path::Path, str::FromStr};

use anyhow::Result;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

static MIGRATOR: Migrator = sqlx::migrate!("./db/migrations");

#[tokio::main]
async fn main() -> Result<()> {
    let current_dir = std::env::current_dir()?;

    setup_db(&current_dir).await?;

    copy_config_to_output()?;

    load_dot_env()?;

    Ok(())
}

async fn setup_db(current_dir: &Path) -> Result<()> {
    println!("cargo:rerun-if-changed=db");

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
    println!("cargo::rustc-env=SQLITE.URL={}", db_url);

    Ok(())
}

fn copy_config_to_output() -> Result<()> {
    println!("cargo:rerun-if-changed=config");

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

        let json = serde_json::from_str::<serde_json::Value>(&fs::read_to_string(path)?)?;
        let contents = serde_json::to_string(&json)?;

        println!("cargo:rustc-env=CONFIG_{}={}", config_name, contents);
        println!("cargo:warning=Set CONFIG_{} with contents", config_name);
    }

    Ok(())
}

fn load_dot_env() -> Result<()> {
    println!("cargo:rerun-if-changed=.env");

    let dot_env_path = std::env::current_dir()?.join(".env");
    let contents = fs::read_to_string(dot_env_path)?;

    for line in contents.lines() {
        let (key, value) = line.split_once('=').unwrap();
        println!("cargo:rustc-env={}={}", key, value);
    }

    Ok(())
}
