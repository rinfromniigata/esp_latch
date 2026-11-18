//! 操作ログ（imple 6章）。メモリ内だけに持ち、ファイルには保存しない。アプリを終了すると消える。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::events::{AppEvent, EventSink};

/// 保持する件数の上限。超えたら古いものから捨てる
pub const LOG_CAPACITY: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogResult {
    Ok,
    Fail,
    /// 認証のダイアログでキャンセルされた
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogEntry {
    /// 記録した時刻（UNIX時刻のミリ秒）
    pub at: u64,
    /// 実行したコマンド（原文のまま見せる）
    pub command: String,
    pub result: LogResult,
}

pub struct OperationLog {
    entries: Mutex<VecDeque<LogEntry>>,
    capacity: usize,
    events: Arc<dyn EventSink>,
}

impl OperationLog {
    pub fn new(events: Arc<dyn EventSink>) -> Self {
        Self::with_capacity(events, LOG_CAPACITY)
    }

    pub fn with_capacity(events: Arc<dyn EventSink>, capacity: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(capacity.min(LOG_CAPACITY))),
            capacity,
            events,
        }
    }

    /// 1行を記録し、`log-appended`を出す
    pub fn append(&self, command: impl Into<String>, result: LogResult) -> LogEntry {
        let entry = LogEntry {
            at: now_ms(),
            command: command.into(),
            result,
        };
        {
            let mut entries = self.entries.lock().expect("log lock");
            while entries.len() >= self.capacity {
                entries.pop_front();
            }
            entries.push_back(entry.clone());
        }
        self.events.emit(AppEvent::LogAppended(entry.clone()));
        entry
    }

    /// 古い順のすべての記録
    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries
            .lock()
            .expect("log lock")
            .iter()
            .cloned()
            .collect()
    }

    /// すべての記録を消し、`log-cleared`を出す
    pub fn clear(&self) {
        self.entries.lock().expect("log lock").clear();
        self.events.emit(AppEvent::LogCleared);
    }
}

/// 現在のUNIX時刻（ミリ秒）
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RecordingSink;

    #[test]
    fn appends_in_order_and_emits() {
        let sink = Arc::new(RecordingSink::default());
        let log = OperationLog::new(sink.clone());
        log.append("diskutil list -plist", LogResult::Ok);
        log.append("diskutil mount disk4s1", LogResult::Fail);
        let commands: Vec<_> = log.entries().into_iter().map(|e| e.command).collect();
        assert_eq!(commands, ["diskutil list -plist", "diskutil mount disk4s1"]);
        assert_eq!(sink.names(), ["log-appended", "log-appended"]);
        assert!(log.entries()[0].at > 0);
    }

    #[test]
    fn drops_oldest_beyond_capacity() {
        let log = OperationLog::with_capacity(Arc::new(RecordingSink::default()), 3);
        for i in 0..5 {
            log.append(format!("cmd {i}"), LogResult::Ok);
        }
        let commands: Vec<_> = log.entries().into_iter().map(|e| e.command).collect();
        assert_eq!(commands, ["cmd 2", "cmd 3", "cmd 4"]);
    }

    #[test]
    fn default_capacity_is_1000() {
        let log = OperationLog::new(Arc::new(RecordingSink::default()));
        for i in 0..1005 {
            log.append(format!("cmd {i}"), LogResult::Ok);
        }
        let entries = log.entries();
        assert_eq!(entries.len(), 1000);
        assert_eq!(entries[0].command, "cmd 5");
    }

    #[test]
    fn clear_empties_and_emits() {
        let sink = Arc::new(RecordingSink::default());
        let log = OperationLog::new(sink.clone());
        log.append("x", LogResult::Cancelled);
        log.clear();
        assert!(log.entries().is_empty());
        assert_eq!(sink.names(), ["log-appended", "log-cleared"]);
    }

    #[test]
    fn serializes_for_the_ui() {
        let entry = LogEntry {
            at: 1,
            command: "open /Volumes/EFI".into(),
            result: LogResult::Cancelled,
        };
        assert_eq!(
            serde_json::to_string(&entry).unwrap(),
            r#"{"at":1,"command":"open /Volumes/EFI","result":"cancelled"}"#
        );
    }
}
