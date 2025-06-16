mod book;

pub use book::*;
use sqlx::Row;

use crate::error::AppResult;
use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::sqlite::Database;

#[derive(Deserialize, Serialize)]
pub struct BookParams {
    title: String,
    author: String,
}
#[debug_handler]
pub async fn create_book(
    State(db): State<Database>,
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
    .fetch_one(db.as_ref())
    .await?
    .id;

    Ok(Json(Book { title, author, id }))
}

#[debug_handler]
pub async fn get_books(State(db): State<Database>) -> AppResult<Json<Vec<Book>>> {
    let books = sqlx::query(
        r#"
        SELECT title, author, id
        FROM books
        ORDER BY id
        "#,
    )
    .fetch_all(db.as_ref())
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
pub async fn get_book_by_id(
    State(db): State<Database>,
    Path(id): Path<i64>,
) -> AppResult<Json<Book>> {
    let book = sqlx::query_as!(Book, "SELECT title, author, id FROM books WHERE id = ?", id)
        .fetch_one(db.as_ref())
        .await?;
    Ok(Json(book))
}

#[derive(Deserialize)]
pub struct FindBookParams {
    title: Option<String>,
    author: Option<String>,
}

#[debug_handler]
pub async fn find_books(
    Query(params): Query<FindBookParams>,
    State(db): State<Database>,
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
    .fetch_all(db.as_ref())
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
mod test {
    use super::*;
    use crate::tests::create_test_server;
    // Test creating a new book
    #[tokio::test]
    async fn test_create_book() {
        let server = create_test_server().await;
        let response = server
            .post("/books/create")
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
        let response = server.get("/books/list").await;
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
            .post("/books/create")
            .json(&BookParams {
                title: "Test Book".to_string(),
                author: "Test Author".to_string(),
            })
            .await;

        // Then get all books
        let response = server.get("/books/list").await;
        assert_eq!(response.status_code(), 200);
        let books: Vec<Book> = response.json();
        assert!(!books.is_empty());
        assert_eq!(books[0].title, "Test Book");
    }

    #[tokio::test]
    async fn test_find_books() {
        let server = create_test_server().await;

        // First create a test book
        server
            .post("/books/create")
            .json(&BookParams {
                title: "Test Book".to_string(),
                author: "Test Author".to_string(),
            })
            .await;

        // Test finding by title
        let response = server
            .get("/books/search")
            .add_query_param("title", "Test Book")
            .await;
        assert_eq!(response.status_code(), 200);
        let books: Vec<Book> = response.json();
        assert!(!books.is_empty());
        assert_eq!(books[0].title, "Test Book");
        assert_eq!(books[0].author, "Test Author");

        // Test finding by author
        let response = server
            .get("/books/search")
            .add_query_param("author", "Test Author")
            .await;
        assert_eq!(response.status_code(), 200);
        let books: Vec<Book> = response.json();
        assert!(!books.is_empty());
        assert_eq!(books[0].title, "Test Book");
        assert_eq!(books[0].author, "Test Author");

        // Test finding non-existent book
        let response = server
            .get("/books/search")
            .add_query_param("title", "Non-existent Book")
            .await;
        assert_eq!(response.status_code(), 404);
    }
}
