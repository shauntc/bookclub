use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection};

use crate::error::AppResult;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Club {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Club {
    pub async fn from_id(id: i64, db: &mut SqliteConnection) -> AppResult<Option<Self>> {
        let club = sqlx::query_as!(
            Club,
            r#"
            SELECT id, name, description, created_at, updated_at
            FROM clubs
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(db)
        .await?;

        Ok(club)
    }
}
