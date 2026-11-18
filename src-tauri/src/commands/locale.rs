//! OSの優先言語の取得。どのロケールを使うかはUI（`src/lib/i18n/`）が決める。

/// OSの優先言語を、優先順にBCP 47の言語タグで返す（例：`["ja-JP", "en-US"]`）。
///
/// WebViewの`navigator.languages`は、アプリのローカライズに引きずられてOSの設定と一致しないことがあるため、
/// OSから直接読む。
#[tauri::command]
pub fn system_locales() -> Vec<String> {
    sys_locale::get_locales().collect()
}
