//! デバイスの一覧（imple 4章）。`diskutil`の実行に1秒ほどかかるため、別スレッドで実行する。

use std::sync::Arc;

use tauri::State;

use crate::app::{AppCore, AppError};
use crate::domain::DeviceSnapshot;

/// デバイスの一覧。取得済みならそれを返す
#[tauri::command]
pub async fn list_devices(core: State<'_, Arc<AppCore>>) -> Result<DeviceSnapshot, AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.list_devices()).await
}

/// 再取得して`devices-changed`を出す（更新ボタン）
#[tauri::command]
pub async fn refresh(core: State<'_, Arc<AppCore>>) -> Result<(), AppError> {
    let core = Arc::clone(&core);
    run_blocking(move || core.refresh().map(drop)).await
}

/// 時間のかかる処理を、非同期のランタイムを止めずに実行する
pub async fn run_blocking<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| AppError::Io {
            message: error.to_string(),
        })?
}
