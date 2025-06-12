use crate::sqlite;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub sqlite: sqlite::Settings,
}
