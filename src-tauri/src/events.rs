//! UIへ送るイベント（imple 4章）。アプリ層は`EventSink`だけに依存し、テストでは記録するだけの実装を渡す。

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::domain::DeviceSnapshot;
use crate::oplog::LogEntry;
use crate::privileged::PrivilegedStatus;
use crate::settings::Settings;

/// 操作の種類（`busy`イベントで、どのボタンを処理中にするかに使う）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    MountEsp,
    UnmountEsp,
    OpenEsp,
    EjectDevice,
    EmptyEspTrash,
}

/// `busy`イベントの中身。操作の開始で`active: true`、終了で`false`
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BusyState {
    /// 対象のBSD名（`disk2s1`・`disk2`）
    pub target: String,
    pub op: Operation,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    DevicesChanged(DeviceSnapshot),
    LogAppended(LogEntry),
    LogCleared,
    SettingsChanged(Settings),
    Busy(BusyState),
    PrivilegedStatus(PrivilegedStatus),
}

impl AppEvent {
    /// UIで`listen`するイベント名
    pub const fn name(&self) -> &'static str {
        match self {
            Self::DevicesChanged(_) => "devices-changed",
            Self::LogAppended(_) => "log-appended",
            Self::LogCleared => "log-cleared",
            Self::SettingsChanged(_) => "settings-changed",
            Self::Busy(_) => "busy",
            Self::PrivilegedStatus(_) => "privileged-status",
        }
    }
}

/// イベントの送り先
pub trait EventSink: Send + Sync {
    fn emit(&self, event: AppEvent);
}

/// すべてのウィンドウへ送る
pub struct TauriEventSink(pub AppHandle);

impl EventSink for TauriEventSink {
    fn emit(&self, event: AppEvent) {
        let name = event.name();
        let result = match event {
            AppEvent::DevicesChanged(snapshot) => self.0.emit(name, snapshot),
            AppEvent::LogAppended(entry) => self.0.emit(name, entry),
            AppEvent::LogCleared => self.0.emit(name, ()),
            AppEvent::SettingsChanged(settings) => self.0.emit(name, settings),
            AppEvent::Busy(state) => self.0.emit(name, state),
            AppEvent::PrivilegedStatus(status) => self.0.emit(name, status),
        };
        if let Err(error) = result {
            eprintln!("emit {name} failed: {error}");
        }
    }
}

/// テスト用：送ったイベントを記録する
#[cfg(test)]
#[derive(Default)]
pub struct RecordingSink(pub std::sync::Mutex<Vec<AppEvent>>);

#[cfg(test)]
impl RecordingSink {
    pub fn names(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().iter().map(AppEvent::name).collect()
    }

    pub fn take(&self) -> Vec<AppEvent> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

#[cfg(test)]
impl EventSink for RecordingSink {
    fn emit(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}
