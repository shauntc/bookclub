use anyhow::Result;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://openlibrary.org/";

const FIELDS: &str = "title,author_name,key";
#[derive(Debug, Deserialize, Serialize)]
pub struct OpenLibBook {
    pub title: String,
    pub author_name: Option<Vec<String>>,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SearchResponse {
    docs: Vec<OpenLibBook>,
}

#[derive(Debug, Clone)]
pub struct OpenLibraryClient {
    client: reqwest::Client,
}

impl OpenLibraryClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn search_book(&self, title: &str) -> Result<Option<OpenLibBook>> {
        let escaped_title = title.replace(' ', "+");
        let url = format!("{BASE_URL}/search.json?q={escaped_title}&fields={FIELDS}");
        tracing::info!("OpenLib URL: {}", url);
        let res = self.client.get(url).send().await?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(anyhow::anyhow!("Failed to fetch book data"));
        }
        let body = res.text().await?;
        let search_res = serde_json::from_str::<SearchResponse>(&body)?;

        Ok(search_res.docs.into_iter().next())
    }
}
