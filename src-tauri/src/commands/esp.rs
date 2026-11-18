//! ESPの操作（imple 4章）。`diskutil`と補助プロセスの応答を待つため、別スレッドで実行する。

use std::sync::Arc;

use tauri::State;

use super::devices::run_blocking;
use crate::app::{AppCore, AppError};
use crate::domain::EspDetail;
use crate::settings::SettingsStore;

/// マウント中のESPの詳細（ブートローダー、容量）
#[tauri::command]
pub async fn inspect_esp(
    core: State<'_, Arc<AppCore>>,
    part: String,
) -> Result<EspDetail, AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.inspect_esp(&part)).await
}

/// ESPをマウントする。設定の`open_finder_after_mount`に従ってFinderで開く
#[tauri::command]
pub async fn mount_esp(
    core: State<'_, Arc<AppCore>>,
    settings: State<'_, Arc<SettingsStore>>,
    part: String,
) -> Result<EspDetail, AppError> {
    let core = Arc::clone(&core);
    let open_finder = settings.get().open_finder_after_mount;
    run_blocking(move || core.mount_esp(&part, open_finder)).await
}

/// マウント中のESPを取り外す
#[tauri::command]
pub async fn unmount_esp(core: State<'_, Arc<AppCore>>, part: String) -> Result<(), AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.unmount_esp(&part)).await
}

/// マウント先を引き直してFinderで開く
#[tauri::command]
pub async fn open_esp(core: State<'_, Arc<AppCore>>, part: String) -> Result<(), AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.open_esp(&part)).await
}
