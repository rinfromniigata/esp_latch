mod commands;

use commands::window::{AppWindow, bring_to_front};
use tauri::{Manager, RunEvent, WindowEvent};

pub fn run() {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::locale::system_locales,
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
