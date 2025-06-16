use crate::{open_library, sqlite};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub sqlite: sqlite::Settings,
    pub open_library: open_library::Settings,
}
