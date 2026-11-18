pub mod app;
pub mod boot;
mod commands;
pub mod disk_info;
pub mod domain;
pub mod events;
pub mod monitor;
pub mod oplog;
pub mod runner;
pub mod settings;

use std::sync::Arc;

use app::AppCore;
use boot::NvramBootInfo;
use commands::window::{AppWindow, bring_to_front};
use disk_info::DiskutilAdapter;
use events::TauriEventSink;
use monitor::{DeviceMonitor, apply_auto_refresh};
use runner::{CommandRunner, SystemRunner};
use settings::{SETTINGS_FILE, SettingsStore};
use tauri::{Manager, RunEvent, WindowEvent};

pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let events = Arc::new(TauriEventSink(app.handle().clone()));
            let settings = Arc::new(SettingsStore::load(
                app.path().app_config_dir()?.join(SETTINGS_FILE),
                events.clone(),
            ));
            let runner: Arc<dyn CommandRunner> = Arc::new(SystemRunner);
            let core = AppCore::new(
                Arc::new(DiskutilAdapter::new(Arc::clone(&runner))),
                Arc::new(NvramBootInfo::new(runner)),
                events,
            );
            let core = Arc::new(core);
            // ディスクの変化を検知したら再取得してdevices-changedを出す。失敗は操作ログに残る
            let monitor = Arc::new(DeviceMonitor::new({
                let core = Arc::clone(&core);
                move || {
                    let _ = core.refresh();
                }
            }));
            if let Err(error) = apply_auto_refresh(monitor.as_ref(), settings.get().auto_refresh) {
                eprintln!("device monitor unavailable: {error}");
            }
            app.manage(core);
            app.manage(monitor);
            app.manage(settings);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::open_repository,
            commands::devices::list_devices,
            commands::devices::refresh,
            commands::locale::system_locales,
            commands::log::clear_log,
            commands::log::get_log,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::window::open_window,
        ])
        .on_window_event(|window, event| {
            // メインウィンドウは閉じても破棄せず隠す（アプリメニューを組み立てる役を持つため）。
            // ⌘Qやメニューの「終了」では、そのまま終了する
            if window.label() == AppWindow::Main.label()
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        // Dockのアイコンを押したとき、見えているウィンドウがなければメインウィンドウを出す
        if let RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } = event
            && let Some(main) = app.get_webview_window(AppWindow::Main.label())
        {
            let _ = bring_to_front(&main);
        }
    });
}
