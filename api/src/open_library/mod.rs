mod implementations;
mod open_library;

pub use open_library::*;

use axum::extract::FromRef;

impl FromRef<crate::AppState> for OpenLibraryClient<reqwest::Client> {
    fn from_ref(app: &crate::AppState) -> Self {
        OpenLibraryClient(app.client.clone())
    }
}
