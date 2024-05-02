use super::OpenLibrary;
use serde::de::DeserializeOwned;

impl OpenLibrary for reqwest::Client {
    type Error = reqwest::Error;

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, Self::Error> {
        self.get(url).send().await?.json().await
    }
}
