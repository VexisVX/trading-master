use crate::risk::live_calc::*;
use tauri::State;

#[tauri::command]
pub fn live_risk_calculate(
    _state: State<'_, crate::commands::AppState>,
    input: LiveCalcInput,
) -> Result<String, String> {
    let r = calculate(&input).map_err(|e| e.to_string())?;
    serde_json::to_string(&r).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn live_partial_close(input: PartialCloseInput) -> Result<String, String> {
    let r = partial_close(&input).map_err(|e| e.to_string())?;
    serde_json::to_string(&r).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn live_martingale_check(input: MartingaleCheckInput) -> Result<String, String> {
    let r = martingale_check(&input);
    serde_json::to_string(&r).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn live_break_even(input: BreakEvenInput) -> Result<String, String> {
    let r = break_even(&input).map_err(|e| e.to_string())?;
    serde_json::to_string(&r).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn live_size_from_risk(input: SizeFromRiskInput) -> Result<String, String> {
    let r = size_from_risk(&input).map_err(|e| e.to_string())?;
    serde_json::to_string(&r).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn live_assistant(input: AssistantInput) -> Result<String, String> {
    let r = assistant(&input).map_err(|e| e.to_string())?;
    serde_json::to_string(&r).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn live_portfolio_risk(input: PortfolioRiskInput) -> Result<String, String> {
    let r = portfolio_risk(&input);
    serde_json::to_string(&r).map_err(|e| e.to_string())
}
