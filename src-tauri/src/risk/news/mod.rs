pub mod calendar;
pub mod finnhub;
pub mod fmp;
pub mod marketaux;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    pub uuid: String,
    pub title: String,
    pub snippet: String,
    pub url: String,
    pub source: String,
    pub published_at: String,
    pub entities: Vec<NewsEntity>,
    pub sentiment: Option<f64>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsEntity {
    pub symbol: String,
    pub name: String,
    pub sentiment_score: Option<f64>,
    pub industry: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsQuery {
    pub symbols: Option<String>, // comma-separated
    pub search: Option<String>,
    pub limit: Option<u32>,       // max 3 on free tier
    pub language: Option<String>, // "en"
    pub min_sentiment: Option<f64>,
    pub max_sentiment: Option<f64>,
}
