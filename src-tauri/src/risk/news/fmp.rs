//! FMP economic calendar — supports historical ranges.
//! Free tier: 250 requests/day.

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FmpError {
    #[error("missing FMP key")] MissingKey,
    #[error("network: {0}")] Network(String),
    #[error("parse: {0}")] Parse(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FmpEvent {
    pub title: String,
    pub country: String,
    pub currency: String,
    pub date: String,
    pub impact: String,
    pub forecast: String,
    pub previous: String,
    pub actual: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct FmpRaw {
    event: String,
    country: String,
    #[serde(default)] currency: String,
    date: String,
    #[serde(default)] impact: String,
    #[serde(default)] estimate: Option<f64>,
    #[serde(default)] previous: Option<f64>,
    #[serde(default)] actual: Option<f64>,
}

pub async fn fetch_range(api_key: &str, from: &str, to: &str) -> Result<Vec<FmpEvent>, FmpError> {
    if api_key.trim().is_empty() { return Err(FmpError::MissingKey); }

    let url = format!(
        "https://financialmodelingprep.com/api/v3/economic_calendar?from={}&to={}&apikey={}",
        from, to, api_key
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| FmpError::Network(e.to_string()))?;

    let resp = client.get(&url).send().await.map_err(|e| FmpError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(FmpError::Network(format!("HTTP {}", resp.status())));
    }

    let raw: Vec<FmpRaw> = resp.json().await.map_err(|e| FmpError::Parse(e.to_string()))?;

    Ok(raw.into_iter().map(|e| FmpEvent {
        title: e.event,
        country: e.country,
        currency: e.currency,
        date: e.date,
        impact: if e.impact.is_empty() { "Low".into() } else { e.impact },
        forecast: e.estimate.map(|v| v.to_string()).unwrap_or_default(),
        previous: e.previous.map(|v| v.to_string()).unwrap_or_default(),
        actual: e.actual.map(|v| v.to_string()).unwrap_or_default(),
        description: String::new(),
    }).collect())
}
