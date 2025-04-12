mod books;
mod error;
mod open_library;

use error::AppResult;
use open_library::OpenLibraryClient;

use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tokio::{net::TcpListener, time::Instant};

use axum::{
    routing::{get, post},
    serve, Router,
};
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    db: sqlx::Pool<sqlx::Sqlite>,
    client: OpenLibraryClient,
}

async fn create_app(db_url: &str) -> Result<Router> {
    match Sqlite::database_exists(db_url).await? {
        true => info!("Database already exists"),
        false => Sqlite::create_database(db_url).await?,
    }

    let db = SqlitePool::connect(db_url).await?;

    sqlx::migrate!("db/migrations").run(&db).await?;

    let client = OpenLibraryClient::new(reqwest::Client::new());
    let app_state = AppState { db, client };

    let app = Router::new()
        .route("/hi", get(|| async { "Hello, World!" }))
        .route("/open-library/search", get(open_library::search_book))
        .route("/books/create", post(books::create_book))
        .route("/books/list", get(books::get_books))
        .route("/books/get/{id}", get(books::get_book_by_id))
        .route("/books/search", get(books::find_books))
        .with_state(app_state);

    Ok(app)
}

#[tokio::main]
async fn main() -> AppResult<()> {
    dotenv::dotenv().ok();
    dotenv::from_path("./api/").ok();

    let start = Instant::now();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");

    let app = create_app(&db_url).await?;

    let port = std::env::var("API_PORT").unwrap_or("3000".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!("Listening on {}", listener.local_addr()?);

    // Create a shutdown signal handler
    let shutdown = async move {
        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = terminate => {},
        }
        let duration = start.elapsed();
        info!("Shutting down gracefully... in {:?}", duration);
    };

    // Start the server with graceful shutdown
    let server = serve(listener, app).with_graceful_shutdown(shutdown);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use axum_test::TestServer;
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub async fn create_test_server() -> TestServer {
        INIT.call_once(|| {
            tracing_subscriber::fmt().init();
        });

        let db_url = "sqlite::memory:";
        let app = create_app(db_url).await.unwrap();

        TestServer::new(app).unwrap()
    }

    // Test the hello world endpoint
    #[tokio::test]
    async fn test_hello_endpoint() {
        let server = create_test_server().await;
        let response = server.get("/hi").await;
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "Hello, World!");
    }
}
