//! アプリ層。Tauriコマンドから呼ぶユースケースを、下位のtrait（`DiskInfo`・`BootInfo`・`EventSink`）だけで組み立てる。
//! Tauriには依存しないため、偽実装を渡して単体テストする。

mod error;
mod locks;

use std::sync::{Arc, Mutex, OnceLock};

pub use error::AppError;
pub use locks::{DiskLock, DiskLocks};

use crate::boot::BootInfo;
use crate::disk_info::DiskInfo;
use crate::domain::DeviceSnapshot;
use crate::events::{AppEvent, BusyState, EventSink, Operation};
use crate::oplog::{LogEntry, LogResult, OperationLog, now_ms};

pub struct AppCore {
    disk_info: Arc<dyn DiskInfo>,
    boot_info: Arc<dyn BootInfo>,
    events: Arc<dyn EventSink>,
    log: OperationLog,
    locks: DiskLocks,
    /// 起動に使ったESPは起動中に変わらないため、最初の取得で1回だけ読む
    boot_part_uuid: OnceLock<Option<String>>,
    /// 最後に取得した一覧。再取得は同時に1つだけにする（ロックを兼ねる）
    snapshot: Mutex<Option<DeviceSnapshot>>,
}

impl AppCore {
    pub fn new(
        disk_info: Arc<dyn DiskInfo>,
        boot_info: Arc<dyn BootInfo>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            disk_info,
            boot_info,
            log: OperationLog::new(Arc::clone(&events)),
            events,
            locks: DiskLocks::default(),
            boot_part_uuid: OnceLock::new(),
            snapshot: Mutex::new(None),
        }
    }

    pub fn log(&self) -> &OperationLog {
        &self.log
    }

    /// デバイスの一覧。取得済みならそれを返し、まだなら取得する
    pub fn list_devices(&self) -> Result<DeviceSnapshot, AppError> {
        let mut snapshot = self.snapshot.lock().expect("snapshot lock");
        if let Some(current) = snapshot.as_ref() {
            return Ok(current.clone());
        }
        let fresh = self.load()?;
        *snapshot = Some(fresh.clone());
        Ok(fresh)
    }

    /// 一覧を取得し直し、`devices-changed`を出す（更新ボタン、ディスクの変化の検知）
    pub fn refresh(&self) -> Result<DeviceSnapshot, AppError> {
        let mut snapshot = self.snapshot.lock().expect("snapshot lock");
        let fresh = self.load()?;
        *snapshot = Some(fresh.clone());
        drop(snapshot);
        self.events.emit(AppEvent::DevicesChanged(fresh.clone()));
        Ok(fresh)
    }

    /// 一覧の取得1回分。操作ログには`diskutil list -plist`の1行と、途中で失敗したコマンドだけを残す
    fn load(&self) -> Result<DeviceSnapshot, AppError> {
        let listing = match self.disk_info.list_devices() {
            Ok(listing) => listing,
            Err(error) => {
                let error = AppError::from(error);
                if let AppError::CommandFailed { command, .. } = &error {
                    self.log.append(command.clone(), LogResult::Fail);
                }
                return Err(error);
            }
        };
        self.log.append(listing.command, LogResult::Ok);
        for failure in listing.failures {
            self.log.append(failure.command, LogResult::Fail);
        }
        Ok(DeviceSnapshot {
            devices: listing.devices,
            boot_part_uuid: self
                .boot_part_uuid
                .get_or_init(|| self.boot_info.boot_part_uuid())
                .clone(),
            taken_at: now_ms(),
        })
    }

    /// 操作を始める。同じディスクを操作中なら`Busy`。`busy`イベントを出し、戻り値を破棄すると終了を出す
    pub fn begin_operation(
        &self,
        target: &str,
        op: Operation,
    ) -> Result<OperationGuard<'_>, AppError> {
        let lock = self.locks.try_lock(target)?;
        self.events.emit(AppEvent::Busy(BusyState {
            target: target.to_owned(),
            op,
            active: true,
        }));
        Ok(OperationGuard {
            core: self,
            target: target.to_owned(),
            op,
            _lock: lock,
        })
    }

    pub fn get_log(&self) -> Vec<LogEntry> {
        self.log.entries()
    }

    pub fn clear_log(&self) {
        self.log.clear();
    }
}

/// 実行中の操作。破棄すると`busy`の終了を出し、ディスクのロックを外す
pub struct OperationGuard<'a> {
    core: &'a AppCore,
    target: String,
    op: Operation,
    _lock: DiskLock,
}

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        self.core.events.emit(AppEvent::Busy(BusyState {
            target: std::mem::take(&mut self.target),
            op: self.op,
            active: false,
        }));
    }
}

#[cfg(test)]
mod tests;
