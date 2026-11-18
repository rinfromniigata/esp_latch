//! 設定（imple 7章）。アプリの設定ディレクトリの`settings.json`に、serdeで直接読み書きする。
//!
//! - ファイルがなければ既定値で始め、最初に設定を変えるまでファイルを作らない
//! - 壊れたファイル・知らない値の項目・ない項目は、その項目だけ既定値にする。次の保存で正しい内容に上書きされる
//! - 変更は1項目ずつ即時に保存し、同じ値への変更は書き込まない。複数のウィンドウからの変更は順に適用し、
//!   適用後の値を`settings-changed`で全ウィンドウに配る
//!
//! Tauriのstoreプラグインは使わない。項目ごとに既定値へ戻す扱いを自前で持つ必要があり、
//! UIから直接ファイルを触らせない（コマンドを通す）方が、変更を1か所で順に適用できるため。

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::events::{AppEvent, EventSink};

/// 設定ファイルの名前
pub const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-Hans")]
    ZhHans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// 起動時のサイドバーの絞り込み
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultFilter {
    #[default]
    All,
    Esp,
    Physical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Settings {
    pub language: Language,
    pub theme: Theme,
    pub default_filter: DefaultFilter,
    pub open_finder_after_mount: bool,
    pub auto_refresh: bool,
    pub confirm_eject: bool,
    pub auto_empty_trash_on_unmount: bool,
    pub show_command: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::System,
            theme: Theme::System,
            default_filter: DefaultFilter::All,
            open_finder_after_mount: true,
            auto_refresh: true,
            confirm_eject: true,
            auto_empty_trash_on_unmount: false,
            show_command: true,
        }
    }
}

impl Settings {
    /// 設定ファイルの内容から作る。読めない項目は既定値にする
    fn from_json(text: &str) -> Self {
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(text) else {
            return Self::default();
        };
        let d = Self::default();
        Self {
            language: field(&map, "language", d.language),
            theme: field(&map, "theme", d.theme),
            default_filter: field(&map, "default_filter", d.default_filter),
            open_finder_after_mount: field(
                &map,
                "open_finder_after_mount",
                d.open_finder_after_mount,
            ),
            auto_refresh: field(&map, "auto_refresh", d.auto_refresh),
            confirm_eject: field(&map, "confirm_eject", d.confirm_eject),
            auto_empty_trash_on_unmount: field(
                &map,
                "auto_empty_trash_on_unmount",
                d.auto_empty_trash_on_unmount,
            ),
            show_command: field(&map, "show_command", d.show_command),
        }
    }

    /// 変更を当てた値を返す
    fn patched(mut self, patch: &SettingsPatch) -> Self {
        let SettingsPatch {
            language,
            theme,
            default_filter,
            open_finder_after_mount,
            auto_refresh,
            confirm_eject,
            auto_empty_trash_on_unmount,
            show_command,
        } = *patch;
        set(&mut self.language, language);
        set(&mut self.theme, theme);
        set(&mut self.default_filter, default_filter);
        set(&mut self.open_finder_after_mount, open_finder_after_mount);
        set(&mut self.auto_refresh, auto_refresh);
        set(&mut self.confirm_eject, confirm_eject);
        set(
            &mut self.auto_empty_trash_on_unmount,
            auto_empty_trash_on_unmount,
        );
        set(&mut self.show_command, show_command);
        self
    }
}

fn field<T: DeserializeOwned>(map: &Map<String, Value>, key: &str, default: T) -> T {
    map.get(key)
        .and_then(|value| T::deserialize(value).ok())
        .unwrap_or(default)
}

fn set<T>(target: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *target = value;
    }
}

/// UIからの変更。指定した項目だけを変える。知らない項目は受け付けない
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    pub language: Option<Language>,
    pub theme: Option<Theme>,
    pub default_filter: Option<DefaultFilter>,
    pub open_finder_after_mount: Option<bool>,
    pub auto_refresh: Option<bool>,
    pub confirm_eject: Option<bool>,
    pub auto_empty_trash_on_unmount: Option<bool>,
    pub show_command: Option<bool>,
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
    events: Arc<dyn EventSink>,
}

impl SettingsStore {
    /// `path`の設定ファイルを読む。ファイルがない・読めないときは既定値で始める（ファイルは作らない）
    pub fn load(path: PathBuf, events: Arc<dyn EventSink>) -> Self {
        let current = std::fs::read_to_string(&path)
            .map(|text| Settings::from_json(&text))
            .unwrap_or_default();
        Self {
            path,
            current: Mutex::new(current),
            events,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self) -> Settings {
        *self.current.lock().expect("settings lock")
    }

    /// 変更を当てて保存し、`settings-changed`を出す。値が変わらなければ書き込まず、イベントも出さない。
    /// 保存に失敗したら、メモリの値も変えずにエラーを返す
    pub fn update(&self, patch: &SettingsPatch) -> io::Result<Settings> {
        let mut current = self.current.lock().expect("settings lock");
        let next = current.patched(patch);
        if next == *current {
            return Ok(next);
        }
        write_atomically(&self.path, &next)?;
        *current = next;
        // ロックを持ったまま出し、イベントの順を適用の順とそろえる（最後のイベントが最終の値になる）
        self.events.emit(AppEvent::SettingsChanged(next));
        Ok(next)
    }
}

/// 一時ファイルに書いてから置き換える（書き込みの途中で終了しても、壊れたファイルを残さない）
fn write_atomically(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    text.push('\n');
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)
}

#[cfg(test)]
mod tests;
