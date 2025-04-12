use crate::Book;
use anyhow::Result;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://openlibrary.org/";

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResponse {
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

    pub async fn search_book(&self, title: &str) -> Result<Book> {
        self.find_book(title)
            .await
            .map(|opt_book| {
                opt_book.unwrap_or_else(|| OpenLibBook {
                    title: title.to_string(),
                    author_name: None,
                    key: format!("/works/{}", title.to_lowercase().replace(' ', "-")),
                })
            })
            .map(|book| book.into())
    }

    async fn find_book(&self, title: &str) -> Result<Option<OpenLibBook>> {
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

const FIELDS: &str = "title,author_name,key";

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenLibBook {
    pub title: String,
    pub author_name: Option<Vec<String>>,
    pub key: String,
}

impl From<OpenLibBook> for crate::Book {
    fn from(value: OpenLibBook) -> Self {
        Self {
            title: value.title,
            author: value
                .author_name
                .map(|names| names.join(", "))
                .unwrap_or_else(|| "Unknown".to_string()),
            id: None,
        }
    }
}
