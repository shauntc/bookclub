use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    client_id: String,
    client_secret: String,
}
