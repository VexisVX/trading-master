//! Finnhub general news provider.
//! Free tier: 60 calls/minute. Provides image + full summary.

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinnhubError {
    #[error("missing Finnhub key")]
    MissingKey,
    #[error("network: {0}")]
    Network(String),
    #[error("parse: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FinnhubArticle {
    pub id: i64,
    pub category: String,
    pub datetime: i64,
    pub headline: String,
    pub image: String,
    pub related: String,
    pub source: String,
    pub summary: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct FinnhubRaw {
    category: String,
    datetime: i64,
    headline: String,
    #[serde(default)]
    image: String,
    #[serde(default)]
    related: String,
    source: String,
    #[serde(default)]
    summary: String,
    url: String,
    #[serde(default)]
    id: Option<i64>,
}

pub async fn fetch_general(api_key: &str) -> Result<Vec<FinnhubArticle>, FinnhubError> {
    if api_key.trim().is_empty() {
        return Err(FinnhubError::MissingKey);
    }

    let url = format!(
        "https://finnhub.io/api/v1/news?category=general&token={}",
        api_key
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("trading-manager/0.1")
        .build()
        .map_err(|e| FinnhubError::Network(e.to_string()))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| FinnhubError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(FinnhubError::Network(format!("HTTP {}", resp.status())));
    }

    let raw: Vec<FinnhubRaw> = resp
        .json()
        .await
        .map_err(|e| FinnhubError::Parse(e.to_string()))?;

    Ok(raw
        .into_iter()
        .enumerate()
        .map(|(i, a)| FinnhubArticle {
            id: a.id.unwrap_or(i as i64),
            category: a.category,
            datetime: a.datetime,
            headline: a.headline,
            image: a.image,
            related: a.related,
            source: a.source,
            summary: a.summary,
            url: a.url,
        })
        .collect())
}
