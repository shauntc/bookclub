use crate::{auth, open_library, sqlite};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub sqlite: sqlite::Settings,
    pub open_library: open_library::Settings,
    pub google_auth: auth::google::Settings,
}
