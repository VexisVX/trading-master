//! Economic calendar via ForexFactory public JSON feed.
//! No API key required. Only current week is available.

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalendarError {
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CalendarEvent {
    pub title: String,
    pub country: String,
    pub currency: String,
    pub date: String,
    pub impact: String, // "High" | "Medium" | "Low" | "Holiday"
    pub forecast: String,
    pub previous: String,
}

#[derive(Debug, Deserialize)]
struct FFEvent {
    title: String,
    country: String,
    #[serde(default)]
    currency: String,
    date: String,
    impact: String,
    #[serde(default)]
    forecast: String,
    #[serde(default)]
    previous: String,
}

pub async fn fetch_calendar() -> Result<Vec<CalendarEvent>, CalendarError> {
    let url = "https://nfs.faireconomy.media/ff_calendar_thisweek.json";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 trading-manager/0.1")
        .build()
        .map_err(|e| CalendarError::Network(e.to_string()))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| CalendarError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(CalendarError::Network(format!("HTTP {}", resp.status())));
    }

    let events: Vec<FFEvent> = resp
        .json()
        .await
        .map_err(|e| CalendarError::Parse(e.to_string()))?;

    Ok(events
        .into_iter()
        .map(|e| CalendarEvent {
            title: e.title,
            country: e.country,
            currency: e.currency,
            date: e.date,
            impact: e.impact,
            forecast: e.forecast,
            previous: e.previous,
        })
        .collect())
}
