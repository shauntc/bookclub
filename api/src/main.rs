mod auth;
mod books;
mod clubs;
mod error;
mod open_library;
mod settings;
mod sqlite;
mod users;

use config::{Config, Environment};
use error::AppResult;
use open_library::OpenLibraryClient;
use settings::Settings;

use anyhow::Result;
use tokio::{net::TcpListener, time::Instant};

use axum::{
    routing::{delete, get, post, put},
    serve, Router,
};
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    db: sqlite::Database,
    open_lib_client: OpenLibraryClient,
    google_client: auth::google::Client,
}

async fn create_app(config: Config) -> Result<Router> {
    let settings = config.try_deserialize::<Settings>()?;
    let db = sqlite::Database::new(&settings.sqlite).await?;

    let google_client =
        auth::google::Client::new("http://127.0.0.1:3000".into(), settings.google_auth).await?;
    let open_lib_client = OpenLibraryClient::new(reqwest::Client::new(), settings.open_library);
    let app_state = AppState {
        db,
        open_lib_client,
        google_client,
    };

    let app = Router::new()
        .route("/hi", get(|| async { "Hello, World!" }))
        .route("/open-library/search", get(open_library::search_book))
        .route("/books/create", post(books::create_book))
        .route("/books/list", get(books::get_books))
        .route("/books/get/{id}", get(books::get_book_by_id))
        .route("/books/search", get(books::find_books))
        .route("/users/create", post(users::create_user))
        .route("/users/list", get(users::get_users))
        .route("/users/{id}", get(users::get_user_by_id))
        .route("/users/{id}", put(users::update_user))
        .route("/users/{id}", delete(users::delete_user))
        .route("/users/search", get(users::find_users))
        .route("/clubs", post(clubs::create_club))
        .route("/clubs/list", get(clubs::get_clubs))
        .route("/clubs/{id}", get(clubs::get_club_by_id))
        .route("/clubs/{id}", put(clubs::update_club))
        .route("/clubs/{id}", delete(clubs::delete_club))
        .route("/memberships", post(clubs::memberships::create_membership))
        .route("/memberships", get(clubs::memberships::get_memberships))
        .route(
            "/memberships/{id}",
            get(clubs::memberships::get_membership_by_id),
        )
        .route(
            "/memberships/{id}",
            delete(clubs::memberships::delete_membership),
        )
        .nest("/auth", auth::router())
        .with_state(app_state);

    Ok(app)
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let start = Instant::now();
    tracing_subscriber::fmt()
        // .with_env_filter(EnvFilter::from_default_env())
        .init();

    warn!("config default: {}", env!("CONFIG_DEFAULT"));

    #[cfg(debug_assertions)]
    let mode_config = option_env!("CONFIG_DEBUG");

    #[cfg(not(debug_assertions))]
    let mode_config = option_env!("CONFIG_RELEASE");

    let mut config_builder = config::Config::builder().add_source(config::File::from_str(
        env!("CONFIG_DEFAULT"),
        config::FileFormat::Json,
    ));

    if let Some(mode_config) = mode_config {
        config_builder = config_builder.add_source(config::File::from_str(
            mode_config,
            config::FileFormat::Json,
        ));
    }

    config_builder = config_builder.add_source(Environment::default().separator("."));

    let config = config_builder.build().expect("Failed to build config");

    let app = create_app(config).await?;

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
    use tracing_test::traced_test;

    pub async fn create_test_server() -> TestServer {
        let default_config = env!("CONFIG_DEFAULT");
        let mode_config = option_env!("CONFIG_TEST");

        let mut config_builder = Config::builder().add_source(config::File::from_str(
            default_config,
            config::FileFormat::Json,
        ));

        if let Some(mode_config) = mode_config {
            config_builder = config_builder.add_source(config::File::from_str(
                mode_config,
                config::FileFormat::Json,
            ));
        }
        config_builder = config_builder
            .set_override("sqlite.url", "sqlite::memory:")
            .expect("Failed to set override");

        let config = config_builder.build().expect("Failed to build config");
        let app = create_app(config).await.unwrap();

        TestServer::new(app).unwrap()
    }

    // Test the hello world endpoint
    #[tokio::test]
    #[traced_test]
    async fn test_hello_endpoint() {
        let server = create_test_server().await;
        let response = server.get("/hi").await;
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "Hello, World!");
    }
}
