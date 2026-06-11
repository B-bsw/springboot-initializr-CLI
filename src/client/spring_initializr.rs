use crate::metadata::{self, Metadata};

const API_URL: &str = "https://api-springboot-initializr.vercel.app/api";

pub struct SpringInitializrClient;

impl SpringInitializrClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch_metadata(&self) -> Result<Metadata, String> {
        let raw: metadata::RawMetadata = reqwest::get(API_URL)
            .await
            .map_err(|e| format!("Failed to fetch metadata: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse metadata: {e}"))?;
        Ok(metadata::map_metadata(raw))
    }

    pub async fn fetch_build_snippet(&self, deps: &[String], boot_version: &str, is_maven: bool) -> Result<String, String> {
        let joined = deps.join(",");
        let url = if is_maven {
            format!("https://start.spring.io/pom.xml?dependencies={}&bootVersion={}", joined, boot_version)
        } else {
            format!("https://start.spring.io/build.gradle?dependencies={}&bootVersion={}", joined, boot_version)
        };
        let resp = reqwest::get(&url).await.map_err(|e| format!("Failed to fetch build snippet: {e}"))?;
        resp.text().await.map_err(|e| format!("Failed to read build snippet: {e}"))
    }
}
