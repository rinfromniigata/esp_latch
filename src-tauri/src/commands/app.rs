//! アプリ自体の情報（バージョン・リポジトリ）と、リポジトリのページを既定のブラウザで開く操作（S02）。

use std::process::{Command, Stdio};

use serde::Serialize;

/// リポジトリのURL。ワークスペースの`Cargo.toml`の値で、UIからは受け取らない（任意のURLを開かせない）
const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub repository: String,
}

pub fn app_info_value() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        repository: REPOSITORY_URL.to_owned(),
    }
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    app_info_value()
}

/// リポジトリのページを既定のブラウザで開く
#[tauri::command]
pub fn open_repository() -> Result<(), String> {
    Command::new("open")
        .arg(REPOSITORY_URL)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|error| {
            eprintln!("open_repository failed: {error}");
            "open_repository_failed".to_owned()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_comes_from_cargo_metadata() {
        let info = app_info_value();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.repository.starts_with("https://"));
    }
}
