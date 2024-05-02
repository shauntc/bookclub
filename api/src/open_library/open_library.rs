use serde::de::DeserializeOwned;

const BASE_URL: &str = "https://openlibrary.org/";

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    num_found: u64,
    docs: Vec<Book>,
    start: u64,
    num_found_exact: bool,
}

pub struct OpenLibraryClient<T: OpenLibrary>(pub T);
impl OpenLibrary for OpenLibraryClient<reqwest::Client> {
    type Error = reqwest::Error;

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, Self::Error> {
        self.0.get(url).send().await?.json().await
    }
}

const FIELDS: &str = "title,author_name,key";
#[derive(Debug, serde::Deserialize)]
pub struct Book {
    pub title: String,
    pub author_name: Vec<String>,
    pub key: String,
}

pub trait OpenLibrary {
    type Error;
    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, Self::Error>;

    async fn find_book(&self, title: &str) -> Result<Option<Book>, Self::Error> {
        let escaped_title = title.replace(' ', "+");
        let url = format!("{BASE_URL}/search.json?q={escaped_title}&fields={FIELDS}");

        let response: SearchResponse = self.get(&url).await?;

        Ok(response.docs.into_iter().next())
    }
}
