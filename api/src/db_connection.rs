use axum::extract::FromRef;

pub struct DbConnection(pub sqlx::SqliteConnection);

impl FromRef<crate::AppState> for DbConnection {
    fn from_ref(app: &crate::AppState) -> Self {
        Self(app.db.acquire().clone())
    }
}
