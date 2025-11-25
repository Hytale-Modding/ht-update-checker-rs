use anyhow::Context;
use bytes::Bytes;
use extension_traits::extension;
use serde::de::DeserializeOwned;

#[extension(pub(crate) trait BetterJsonError)]
impl reqwest::Response {
    async fn json_with_error<T>(self) -> anyhow::Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.url().clone();
        let content: Bytes = self
            .bytes()
            .await
            .with_context(|| format!("Unable to read response body for {url}"))?;
        let reader = &mut serde_json::Deserializer::from_slice(&content);
        let json = serde_path_to_error::deserialize(reader)
            .with_context(|| format!("Unable to decode response for {url}"))?;

        Ok(json)
    }
}
