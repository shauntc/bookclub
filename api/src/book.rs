use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection};

use crate::error::AppResult;
use crate::open_library;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: Option<String>,
}
impl Book {
    pub fn from_open_library(
        open_library::Book {
            title, author_name, ..
        }: open_library::Book,
    ) -> Self {
        Self {
            title,
            author: author_name.into_iter().next(),
        }
    }

    pub async fn from_id(id: i64, db: &mut SqliteConnection) -> AppResult<Option<Self>> {
        let book = sqlx::query_as!(
            Book,
            r#"
            SELECT title, author
            FROM books
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(db)
        .await?;

        Ok(book)
    }
}
