mod analytics;
mod commands;
mod db;
mod domain;
mod equity;
mod risk;
mod utils;

use commands::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("app data dir");
            std::fs::create_dir_all(&app_dir).expect("create app data dir");
            let db_path = app_dir.join("app.db");
            let conn = db::init_db(&db_path).expect("init db");
            let bt_path = db_path.parent().unwrap_or(&app_dir).join("backtest.db");
            let bt_conn = db::backtest::init_backtest_db(&bt_path).expect("init backtest db");
            app.manage(AppState {
                db: Mutex::new(conn),
                backtest_db: Mutex::new(bt_conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_schema_version,
            commands::get_balance_placeholder,
            commands::risk::risk_position_size,
            commands::risk::risk_r_multiple,
            commands::risk::risk_echo_decimal,
            commands::trades::trades_create,
            commands::equity::equity_config_get,
            commands::equity::equity_config_set,
            commands::equity::equity_snapshot_create,
            commands::equity::equity_snapshot_latest,
            commands::equity::equity_evaluate,
            commands::equity::equity_pre_evaluate,
            commands::equity::equity_preview_risk,
            commands::equity::equity_list_accounts,
            commands::equity::equity_list_snapshots,
            commands::equity::equity_delete_snapshot,
            commands::equity::equity_clear_snapshots,
            commands::equity::equity_cleanup_old_snapshots,
            commands::equity::equity_auto_snapshot_create,
            commands::equity::equity_decisions_list,
            commands::equity::equity_decision_record,
            commands::backtest::backtest_trades_list,
            commands::backtest::backtest_trade_create,
            commands::backtest::backtest_trade_delete,
            commands::backtest::backtest_reset,
            commands::backtest::backtest_setup_tags,
            commands::backtest::backtest_symbols,
            commands::backtest::backtest_stats,
            
            
            
            
            
            commands::trades::trades_list,
            commands::trades::trades_get,
            commands::trades::trades_add_exit,
            commands::trades::trades_partial_close,
            commands::trades::trades_quick_create,
            commands::trades::trades_sl_history,
            commands::trades::trades_copy_screenshot,
            commands::trades::export_text_file,
            commands::trades::trades_log_sl_move,
            commands::trades::trades_default_account_id,
            commands::trades::trades_remove_attachment,
            commands::trades::trades_add_attachment,
            commands::trades::trades_export_csv,
            commands::trades::trades_delete,
            commands::trades::trades_update,
            commands::analytics::analytics_summary,
            commands::analytics::analytics_recent_r,
            commands::analytics::analytics_breakdown,
            commands::backup::backup_to_file,
            commands::backup::backup_validate,
            commands::backup::backup_restore,
            commands::backup::backup_dir,
            commands::backup::backup_now,
            commands::accounts::accounts_list,
            commands::live_calc::live_risk_calculate,
            commands::live_calc::live_assistant,
            commands::live_calc::live_size_from_risk,
            commands::live_calc::live_portfolio_risk,
            commands::live_presets::drafts_delete,
            commands::news::news_test_key,
            commands::news::calendar_fetch,
            commands::news::fmp_calendar_fetch,
            commands::news::finnhub_news,
            commands::settings::settings_delete,
            commands::goals::goals_update,
            commands::goals::goals_get,
            commands::storage::storage_stats,
            commands::settings::settings_set,
            commands::settings::settings_get,
            commands::news::news_fetch,
            commands::live_presets::drafts_get,
            commands::live_presets::drafts_create,
            commands::live_presets::presets_delete,
            commands::live_presets::presets_create,
            commands::live_presets::presets_list,
            commands::live_calc::live_break_even,
            commands::live_calc::live_martingale_check,
            commands::live_calc::live_partial_close,
            commands::accounts::accounts_create,
            commands::accounts::accounts_delete,
            commands::accounts::accounts_active_get,
            commands::accounts::accounts_active_set,
            commands::strategies::strategies_create,
            commands::strategies::strategies_list,
            commands::strategies::strategies_get,
            commands::strategies::strategies_delete,
            commands::strategies::strategies_update,
            commands::strategies::strategies_get_version,
            commands::trading_plans::plans_delete,
            commands::trading_plans::plans_update,
            commands::trading_plans::plans_create,
            commands::trading_plans::plans_list,
            commands::strategies::strategies_list_versions,
            commands::risk_settings::risk_settings_get,
            commands::risk_settings::risk_settings_update,
            commands::risk_settings::risk_status,
            commands::prop_firm::prop_firm_get,
            commands::prop_firm::prop_firm_update,
            commands::prop_firm::prop_firm_status,
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
