use axum::extract::FromRef;
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};
use tokio::runtime::Handle;

pub struct DbConnection(pub PoolConnection<Sqlite>);

impl FromRef<crate::AppState> for DbConnection {
    fn from_ref(app: &crate::AppState) -> Self {
        Handle::current().block_on(async { Self(app.db.acquire().await.unwrap()) })
    }
}
