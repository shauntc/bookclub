pub mod google;

use axum::{extract::FromRef, routing::get, Router};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/google/login", get(google::login))
        .route("/google/callback", get(google::callback))
}

impl FromRef<AppState> for google::Client {
    fn from_ref(state: &AppState) -> Self {
        state.google_client.clone()
    }
}
