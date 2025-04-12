mod book;
mod db_connection;
mod error;
mod open_library;

use book::Book;
use error::AppResult;
use open_library::OpenLibraryClient;

use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tokio::net::TcpListener;

use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    serve, Json, Router,
};
use serde::Deserialize;
use serde::Serialize;
use sqlx::Row;
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
        .route("/book", get(search_book))
        .route("/book", post(create_book))
        .route("/books", get(get_books))
        .route("/books/{id}", get(get_book_by_id))
        .route("/books/find", get(find_books))
        .with_state(app_state);

    Ok(app)
}

#[tokio::main]
async fn main() -> AppResult<()> {
    dotenv::from_path("./api/").ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");

    let app = create_app(&db_url).await?;

    let port = std::env::var("API_PORT").unwrap_or("3000".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!("Listening on {}", listener.local_addr()?);
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
    State(state): State<AppState>,
) -> Response {
    match state.client.search_book(&title).await {
        Ok(Some(book)) => (StatusCode::OK, Json(book)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Book not found").into_response(),
        Err(e) => {
            tracing::error!("Error searching for book: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error searching for book",
            )
                .into_response()
        }
    }
}

#[derive(Deserialize, Serialize)]
struct BookParams {
    title: String,
    author: String,
}
#[debug_handler]
async fn create_book(
    State(state): State<AppState>,
    Json(BookParams { title, author }): Json<BookParams>,
) -> AppResult<impl IntoResponse> {
    let id = sqlx::query!(
        r#"
        INSERT INTO books (title, author)
        VALUES (?, ?)
        RETURNING id
        "#,
        title,
        author
    )
    .fetch_one(&state.db)
    .await?
    .id;

    Ok(Json(Book { title, author, id }))
}

#[debug_handler]
async fn get_books(State(state): State<AppState>) -> AppResult<Json<Vec<Book>>> {
    let books = sqlx::query(
        r#"
        SELECT title, author, id
        FROM books
        ORDER BY id
        "#,
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|row| Book {
        title: row.get("title"),
        author: row.get("author"),
        id: row.get("id"),
    })
    .collect::<Vec<_>>();

    Ok(Json(books))
}

#[debug_handler]
async fn get_book_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Book>> {
    let book = sqlx::query_as!(Book, "SELECT title, author, id FROM books WHERE id = ?", id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(book))
}

#[derive(Deserialize)]
struct FindBookParams {
    title: Option<String>,
    author: Option<String>,
}

#[debug_handler]
async fn find_books(
    Query(params): Query<FindBookParams>,
    State(state): State<AppState>,
) -> Response {
    if params.title.is_none() && params.author.is_none() {
        return (StatusCode::BAD_REQUEST, "No search parameters provided").into_response();
    }

    let db_result = sqlx::query_as!(
        Book,
        "SELECT title, author, id FROM books WHERE title = ? OR author = ?",
        params.title,
        params.author
    )
    .fetch_all(&state.db)
    .await;

    match db_result {
        Ok(books) => {
            if books.is_empty() {
                (StatusCode::NOT_FOUND, "No books found").into_response()
            } else {
                (StatusCode::OK, Json(books)).into_response()
            }
        }
        Err(e) => {
            tracing::error!("Error fetching books: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error fetching books").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use std::sync::Once;

    static INIT: Once = Once::new();

    async fn create_test_server() -> TestServer {
        INIT.call_once(|| {
            tracing_subscriber::fmt().init();
        });

        let db_url = "sqlite::memory:";
        let app = create_app(db_url).await.unwrap();

        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_search_book() {
        let server = create_test_server().await;
        tracing::warn!("Started test server");

        let response = server
            .get("/book")
            .add_query_param("title", "The Hobbit")
            .await;

        tracing::warn!("Response: {:?}", response);

        assert_eq!(response.status_code(), 200);
        let book: open_library::OpenLibBook = response.json();
        assert_eq!(book.title, "The Hobbit");
        assert_eq!(book.author_name.unwrap()[0], "J.R.R. Tolkien");
    }

    // Test the hello world endpoint
    #[tokio::test]
    async fn test_hello_endpoint() {
        let server = create_test_server().await;
        let response = server.get("/hi").await;
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "Hello, World!");
    }

    // Test creating a new book
    #[tokio::test]
    async fn test_create_book() {
        let server = create_test_server().await;
        let response = server
            .post("/book")
            .json(&BookParams {
                title: "Test Book".to_string(),
                author: "Test Author".to_string(),
            })
            .await;

        assert_eq!(response.status_code(), 200);
        let book: Book = response.json();
        assert_eq!(book.title, "Test Book");
        assert_eq!(book.author, "Test Author");

        // Test getting the book we just created
        let response = server.get("/books").await;
        assert_eq!(response.status_code(), 200);
        let books: Vec<Book> = response.json();
        assert!(!books.is_empty());
        assert_eq!(books[0].title, "Test Book");
        assert_eq!(books[0].author, "Test Author");
    }

    // Test getting all books
    #[tokio::test]
    async fn test_get_books() {
        let server = create_test_server().await;

        // First create a book
        server
            .post("/book")
            .json(&BookParams {
                title: "Test Book".to_string(),
                author: "Test Author".to_string(),
            })
            .await;

        // Then get all books
        let response = server.get("/books").await;
        assert_eq!(response.status_code(), 200);
        let books: Vec<Book> = response.json();
        assert!(!books.is_empty());
        assert_eq!(books[0].title, "Test Book");
    }

    // Test searching for a non-existent book
    #[tokio::test]
    async fn test_search_nonexistent_book() {
        let server = create_test_server().await;
        let response = server
            .get("/book")
            .add_query_param("title", "Nonexistent Book Title That Should Not Exist")
            .await;

        assert_eq!(response.status_code(), 404);
    }

    #[tokio::test]
    async fn test_find_books() {
        let server = create_test_server().await;

        // First create a test book
        server
            .post("/book")
            .json(&BookParams {
                title: "Test Book".to_string(),
                author: "Test Author".to_string(),
            })
            .await;

        // Test finding by title
        let response = server
            .get("/books/find")
            .add_query_param("title", "Test Book")
            .await;
        assert_eq!(response.status_code(), 200);
        let books: Vec<Book> = response.json();
        assert!(!books.is_empty());
        assert_eq!(books[0].title, "Test Book");
        assert_eq!(books[0].author, "Test Author");

        // Test finding by author
        let response = server
            .get("/books/find")
            .add_query_param("author", "Test Author")
            .await;
        assert_eq!(response.status_code(), 200);
        let books: Vec<Book> = response.json();
        assert!(!books.is_empty());
        assert_eq!(books[0].title, "Test Book");
        assert_eq!(books[0].author, "Test Author");

        // Test finding with no parameters (should return 400)
        let response = server.get("/books/find").await;
        assert_eq!(response.status_code(), 400);

        // Test finding non-existent book
        let response = server
            .get("/books/find")
            .add_query_param("title", "Non-existent Book")
            .await;
        assert_eq!(response.status_code(), 404);
    }
}
