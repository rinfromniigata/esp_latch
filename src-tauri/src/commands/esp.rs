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

/// マウント中のESPを取り外す。`empty_trash`か、設定の`auto_empty_trash_on_unmount`なら、先にゴミ箱を空にする
#[tauri::command]
pub async fn unmount_esp(
    core: State<'_, Arc<AppCore>>,
    settings: State<'_, Arc<SettingsStore>>,
    part: String,
    empty_trash: bool,
) -> Result<(), AppError> {
    let core = Arc::clone(&core);
    let empty_trash = empty_trash || settings.get().auto_empty_trash_on_unmount;
    run_blocking(move || core.unmount_esp(&part, empty_trash)).await
}

/// マウント中のESPのゴミ箱を空にする。解放したバイト数を返す
#[tauri::command]
pub async fn empty_esp_trash(core: State<'_, Arc<AppCore>>, part: String) -> Result<u64, AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.empty_esp_trash(&part)).await
}

/// デバイスごと取り出す
#[tauri::command]
pub async fn eject_device(core: State<'_, Arc<AppCore>>, disk: String) -> Result<(), AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.eject_device(&disk)).await
}

/// マウント先を引き直してFinderで開く
#[tauri::command]
pub async fn open_esp(core: State<'_, Arc<AppCore>>, part: String) -> Result<(), AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.open_esp(&part)).await
}
