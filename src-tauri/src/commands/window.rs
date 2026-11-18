//! アプリのウィンドウ（メイン・設定・操作ログ）を開く。すでに開いていれば、2つ目は作らず前面に出す。

use serde::Deserialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// アプリのウィンドウの種類。ラベル・URL・初期サイズを持つ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppWindow {
    Main,
    Settings,
    Log,
}

impl AppWindow {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Settings => "settings",
            Self::Log => "log",
        }
    }

    const fn path(self) -> &'static str {
        match self {
            Self::Main => "/",
            Self::Settings => "/settings",
            Self::Log => "/log",
        }
    }

    /// 初期サイズ（論理ピクセル）。メインはtauri.conf.jsonの値と同じ。設定と操作ログはモックの幅に合わせる
    const fn size(self) -> (f64, f64) {
        match self {
            Self::Main => (1100.0, 720.0),
            Self::Settings => (640.0, 560.0),
            Self::Log => (820.0, 520.0),
        }
    }
}

/// ウィンドウを前面に出す（しまってあれば戻し、隠してあれば表示する）
pub fn bring_to_front(window: &WebviewWindow) -> tauri::Result<()> {
    window.unminimize()?;
    window.show()?;
    window.set_focus()
}

/// ウィンドウを開く。`title`は、UIが辞書から引いた現在のロケールのタイトル
///
/// Windowsでは同期コマンドでウィンドウを作るとデッドロックすることがあるため、非同期にしている（Tauriの推奨）
#[tauri::command]
pub async fn open_window(app: AppHandle, window: AppWindow, title: String) -> Result<(), String> {
    open(&app, window, &title).map_err(|error| {
        eprintln!("open_window({window:?}) failed: {error}");
        "window_open_failed".to_owned()
    })
}

fn open(app: &AppHandle, window: AppWindow, title: &str) -> tauri::Result<()> {
    if let Some(existing) = app.get_webview_window(window.label()) {
        return bring_to_front(&existing);
    }
    let (width, height) = window.size();
    let created =
        WebviewWindowBuilder::new(app, window.label(), WebviewUrl::App(window.path().into()))
            .title(title)
            .inner_size(width, height)
            .build()?;
    created.set_focus()
}

#[cfg(test)]
mod tests {
    use super::AppWindow;

    #[test]
    fn deserializes_from_snake_case() {
        let window: AppWindow = serde_json::from_str("\"settings\"").unwrap();
        assert_eq!(window, AppWindow::Settings);
    }

    #[test]
    fn labels_are_unique() {
        let labels = [AppWindow::Main, AppWindow::Settings, AppWindow::Log].map(AppWindow::label);
        assert_eq!(labels, ["main", "settings", "log"]);
    }
}
