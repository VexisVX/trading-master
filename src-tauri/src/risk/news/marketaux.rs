//! Marketaux news provider.
//!
//! Free tier: 100 requests/day, 3 articles per request.
//! Auth: api_token query parameter.
//! Docs: https://www.marketaux.com/documentation

use super::{NewsArticle, NewsEntity, NewsQuery};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NewsError {
    #[error("missing API key")]
    MissingKey,
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("rate limit exceeded")]
    RateLimit,
}

#[derive(Debug, Deserialize)]
struct MarketauxResponse {
    data: Vec<MarketauxArticle>,
}

#[derive(Debug, Deserialize)]
struct MarketauxArticle {
    uuid: String,
    title: String,
    description: Option<String>,
    snippet: Option<String>,
    url: String,
    source: String,
    published_at: String,
    #[serde(default)]
    entities: Vec<MarketauxEntity>,
    #[serde(default)]
    image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarketauxEntity {
    symbol: String,
    name: String,
    #[serde(default)]
    sentiment_score: Option<f64>,
    #[serde(default)]
    industry: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

pub async fn fetch(api_key: &str, query: &NewsQuery) -> Result<Vec<NewsArticle>, NewsError> {
    if api_key.trim().is_empty() {
        return Err(NewsError::MissingKey);
    }

    let mut url = String::from("https://api.marketaux.com/v1/news/all");
    let mut params: Vec<String> = vec![format!("api_token={}", api_key)];

    // Free tier caps at 3 per request — enforce
    let limit = query.limit.unwrap_or(3).min(3);
    params.push(format!("limit={}", limit));

    if let Some(s) = &query.symbols {
        if !s.trim().is_empty() {
            params.push(format!("symbols={}", s.trim()));
        }
    }
    if let Some(s) = &query.search {
        if !s.trim().is_empty() {
            params.push(format!("search={}", url_encode(s.trim())));
        }
    }
    if let Some(l) = &query.language {
        params.push(format!("language={}", l));
    }
    if let Some(v) = query.min_sentiment {
        params.push(format!("sentiment_gte={}", v));
    }
    if let Some(v) = query.max_sentiment {
        params.push(format!("sentiment_lte={}", v));
    }

    url.push('?');
    url.push_str(&params.join("&"));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("trading-manager/0.1")
        .build()
        .map_err(|e| NewsError::Network(e.to_string()))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| NewsError::Network(e.to_string()))?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::PAYMENT_REQUIRED
    {
        return Err(NewsError::RateLimit);
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(NewsError::Network(format!("HTTP {} — {}", status, &body[..body.len().min(200)])));
    }

    let parsed: MarketauxResponse = resp
        .json()
        .await
        .map_err(|e| NewsError::Parse(e.to_string()))?;

    let articles = parsed.data.into_iter().map(|a| {
        let avg_sentiment = if a.entities.is_empty() {
            None
        } else {
            let sum: f64 = a.entities.iter().filter_map(|e| e.sentiment_score).sum();
            let count = a.entities.iter().filter(|e| e.sentiment_score.is_some()).count();
            if count > 0 { Some(sum / count as f64) } else { None }
        };

        NewsArticle {
            uuid: a.uuid,
            title: a.title,
            snippet: a.snippet.or(a.description).unwrap_or_default(),
            url: a.url,
            source: a.source,
            published_at: a.published_at,
            entities: a.entities.into_iter().map(|e| NewsEntity {
                symbol: e.symbol,
                name: e.name,
                sentiment_score: e.sentiment_score,
                industry: e.industry,
                country: e.country,
            }).collect(),
            sentiment: avg_sentiment,
            image_url: a.image_url,
        }
    }).collect();

    Ok(articles)
}

fn url_encode(s: &str) -> String {
    s.chars().map(|c| match c {
        ' ' => "%20".to_string(),
        '&' => "%26".to_string(),
        '=' => "%3D".to_string(),
        '?' => "%3F".to_string(),
        '#' => "%23".to_string(),
        c => c.to_string(),
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_rejected() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(fetch("", &NewsQuery {
            symbols: None, search: None, limit: None,
            language: None, min_sentiment: None, max_sentiment: None,
        }));
        assert!(matches!(r, Err(NewsError::MissingKey)));
    }
}
