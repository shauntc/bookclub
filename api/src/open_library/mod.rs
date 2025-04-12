mod client;

pub use client::*;

use crate::AppState;
use axum::{
    debug_handler,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    title: String,
}
#[debug_handler]
pub async fn search_book(
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::create_test_server;
    // Test creating a new book

    #[tokio::test]
    async fn test_search_book() {
        let server = create_test_server().await;
        tracing::warn!("Started test server");

        let response = server
            .get("/open-library/search")
            .add_query_param("title", "The Hobbit")
            .await;

        tracing::warn!("Response: {:?}", response);

        assert_eq!(response.status_code(), 200);
        let book: OpenLibBook = response.json();
        assert_eq!(book.title, "The Hobbit");
        assert_eq!(book.author_name.unwrap()[0], "J.R.R. Tolkien");
    }

    #[tokio::test]
    async fn test_search_nonexistent_book() {
        let server = create_test_server().await;
        let response = server
            .get("/open-library/search")
            .add_query_param("title", "Nonexistent Book Title That Should Not Exist")
            .await;

        assert_eq!(response.status_code(), 404);
    }
}
