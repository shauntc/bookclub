use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection};

use crate::error::AppResult;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub id: i64,
}

impl Book {
    pub async fn from_id(id: i64, db: &mut SqliteConnection) -> AppResult<Option<Self>> {
        let book = sqlx::query_as!(
            Book,
            r#"
            SELECT title, author, id
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
