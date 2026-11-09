use crate::commands::AppState;
use crate::db::app_settings as settings_repo;
use crate::risk::news::marketaux;
use crate::risk::news::{NewsArticle, NewsQuery};
use tauri::State;

#[tauri::command]
pub async fn news_fetch(
    state: State<'_, AppState>,
    symbols: Option<String>,
    search: Option<String>,
    min_sentiment: Option<f64>,
    max_sentiment: Option<f64>,
) -> Result<Vec<NewsArticle>, String> {
    // Cache key based on filters
    let cache_key = format!(
        "news_cache_{}_{}_{}_{}",
        symbols.as_deref().unwrap_or(""),
        search.as_deref().unwrap_or(""),
        min_sentiment.map(|v| v.to_string()).unwrap_or_default(),
        max_sentiment.map(|v| v.to_string()).unwrap_or_default(),
    );

    // Check cache (30 min TTL)
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        if let Some(cached) =
            crate::db::app_settings::get_setting(&conn, &cache_key).map_err(|e| e.to_string())?
        {
            let ts_key = format!("{}_ts", cache_key);
            if let Some(ts) =
                crate::db::app_settings::get_setting(&conn, &ts_key).map_err(|e| e.to_string())?
            {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&ts) {
                    let age = chrono::Utc::now().signed_duration_since(t);
                    if age.num_minutes() < 120 {
                        if let Ok(articles) = serde_json::from_str::<Vec<NewsArticle>>(&cached) {
                            return Ok(articles);
                        }
                    }
                }
            }
        }
    }

    let api_key = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::db::app_settings::get_setting(&conn, "marketaux_api_key")
            .map_err(|e| e.to_string())?
            .unwrap_or_default()
    };

    if api_key.trim().is_empty() {
        return Err("No Marketaux API key configured. Go to Settings → Integrations.".into());
    }

    let query = NewsQuery {
        symbols,
        search,
        limit: Some(3),
        language: Some("en".into()),
        min_sentiment,
        max_sentiment,
    };

    let articles = marketaux::fetch(&api_key, &query)
        .await
        .map_err(|e| e.to_string())?;

    // Save to cache
    if let Ok(json) = serde_json::to_string(&articles) {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        let _ = crate::db::app_settings::set_setting(&conn, &cache_key, &json);
        let _ = crate::db::app_settings::set_setting(&conn, &format!("{}_ts", cache_key), &now);
    }

    Ok(articles)
}

#[tauri::command]
pub async fn news_test_key(state: State<'_, AppState>) -> Result<bool, String> {
    let api_key = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        settings_repo::get_setting(&conn, "marketaux_api_key")
            .map_err(|e| e.to_string())?
            .unwrap_or_default()
    };

    if api_key.trim().is_empty() {
        return Err("No API key saved.".into());
    }

    let query = NewsQuery {
        symbols: None,
        search: None,
        limit: Some(1),
        language: Some("en".into()),
        min_sentiment: None,
        max_sentiment: None,
    };

    match marketaux::fetch(&api_key, &query).await {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Test failed: {}", e)),
    }
}

#[tauri::command]
pub async fn calendar_fetch() -> Result<Vec<crate::risk::news::calendar::CalendarEvent>, String> {
    crate::risk::news::calendar::fetch_calendar()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fmp_calendar_fetch(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<Vec<crate::risk::news::fmp::FmpEvent>, String> {
    let api_key = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::db::app_settings::get_setting(&conn, "fmp_api_key")
            .map_err(|e| e.to_string())?
            .unwrap_or_default()
    };
    if api_key.trim().is_empty() {
        return Err("No FMP API key. Set it in Settings → Integrations.".into());
    }
    crate::risk::news::fmp::fetch_range(&api_key, &from, &to)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn finnhub_news(
    state: State<'_, AppState>,
) -> Result<Vec<crate::risk::news::finnhub::FinnhubArticle>, String> {
    let api_key = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::db::app_settings::get_setting(&conn, "finnhub_api_key")
            .map_err(|e| e.to_string())?
            .unwrap_or_default()
    };
    if api_key.trim().is_empty() {
        return Err("No Finnhub API key. Add it in Settings → Integrations.".into());
    }
    crate::risk::news::finnhub::fetch_general(&api_key)
        .await
        .map_err(|e| e.to_string())
}
