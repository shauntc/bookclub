mod book;
mod db_connection;
mod error;
mod open_library;

use book::Book;
use error::AppResult;
use open_library::{OpenLibrary, OpenLibraryClient};

use sqlx::{migrate::MigrateDatabase, query_as, Sqlite, SqlitePool};
use tokio::net::TcpListener;

use axum::{
    debug_handler,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    serve, Json, Router,
};
use serde::Deserialize;
use tracing::info;
use tracing_subscriber::EnvFilter;

struct AppState {
    db: sqlx::Pool<sqlx::Sqlite>,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    dotenv::from_path("./api/").ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");

    match Sqlite::database_exists(&db_url).await? {
        true => info!("Database already exists"),
        false => Sqlite::create_database(&db_url).await?,
    }

    let db = SqlitePool::connect(&db_url).await?;

    sqlx::migrate!("db/migrations").run(&db).await?;

    let client = reqwest::Client::new();

    let app_state = AppState { db, client };

    let app = Router::new()
        .route("/hi", get(|| async { "Hello, World!" }))
        .route("/books", get(get_books))
        .route("/book", get(search_book))
        .with_state(app_state);

    let port = std::env::var("API_PORT").unwrap_or("3000".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    serve(listener, app).await?;

    Ok(())
}

#[derive(Deserialize)]
struct Params {
    title: String,
}
#[debug_handler]
async fn search_book(
    Query(Params { title }): Query<Params>,
    State(client): State<reqwest::Client>,
) -> AppResult<impl IntoResponse> {
    let book = client.find_book(&title).await?;

    match book {
        Some(book) => Ok(serde_json::to_string(&Book::from_open_library(book))?),
        None => Ok("No book found".to_string()),
    }
}

#[derive(Deserialize)]
struct BookParams {
    title: String,
    author: Option<String>,
}
#[debug_handler(state = AppState)]
async fn create_book(
    State(db): State<SqlitePool>,
    State(client): State<OpenLibraryClient<reqwest::Client>>,
    Json(BookParams { title, author }): Json<BookParams>,
) -> AppResult<impl IntoResponse> {
    let book = client.find_book(&title).await?;

    sqlx::query!(
        r#"
        INSERT INTO books (title, author)
        VALUES (?, ?)
        "#,
        title,
        author
    )
    .execute(&db)
    .await?;

    Ok("Book created")
}

async fn get_books(State(db): State<SqlitePool>) -> AppResult<Json<Vec<Book>>> {
    let books = query_as::<_, Book>("select * from books")
        .fetch_all(&db)
        .await?;

    Ok(Json(books))
}
