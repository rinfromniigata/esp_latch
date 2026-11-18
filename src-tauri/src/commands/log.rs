//! 操作ログ（imple 4章・6章）

use std::sync::Arc;

use tauri::State;

use crate::app::AppCore;
use crate::oplog::LogEntry;

#[tauri::command]
pub fn get_log(core: State<'_, Arc<AppCore>>) -> Vec<LogEntry> {
    core.get_log()
}

/// すべての記録を消し、`log-cleared`を出す
#[tauri::command]
pub fn clear_log(core: State<'_, Arc<AppCore>>) {
    core.clear_log();
}
