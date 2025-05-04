use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection};

use crate::error::AppResult;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Membership {
    pub id: i64,
    pub user_id: i64,
    pub club_id: i64,
    pub permission_level: i64,
    pub created_at: NaiveDateTime,
}

impl Membership {
    pub async fn from_id(id: i64, db: &mut SqliteConnection) -> AppResult<Option<Self>> {
        let membership = sqlx::query_as!(
            Membership,
            r#"
            SELECT id, user_id, club_id, permission_level, created_at
            FROM memberships
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(db)
        .await?;

        Ok(membership)
    }
}
