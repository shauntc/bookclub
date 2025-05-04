use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};

#[derive(Debug, Serialize, Deserialize, FromRow, Type)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
