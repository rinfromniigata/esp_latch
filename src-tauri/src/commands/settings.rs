//! 設定（imple 4章・7章）

use std::sync::Arc;

use tauri::State;

use crate::app::{AppCore, AppError};
use crate::monitor::{DeviceMonitor, apply_auto_refresh};
use crate::settings::{Settings, SettingsPatch, SettingsStore};

#[tauri::command]
pub fn get_settings(store: State<'_, Arc<SettingsStore>>) -> Settings {
    store.get()
}

/// 指定した項目だけを変えて保存し、`settings-changed`を出す。`auto_refresh`は監視の開始・停止に反映する
#[tauri::command]
pub fn update_settings(
    patch: SettingsPatch,
    store: State<'_, Arc<SettingsStore>>,
    monitor: State<'_, Arc<DeviceMonitor>>,
    core: State<'_, Arc<AppCore>>,
) -> Result<Settings, AppError> {
    let settings = store.update(&patch)?;
    match apply_auto_refresh(monitor.as_ref(), settings.auto_refresh) {
        // 止めていたあいだの変化を反映する
        Ok(true) => {
            let core = Arc::clone(&core);
            std::thread::spawn(move || {
                let _ = core.refresh();
            });
        }
        Ok(false) => {}
        Err(error) => eprintln!("device monitor unavailable: {error}"),
    }
    Ok(settings)
}
