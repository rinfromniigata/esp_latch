use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::disk_info::{DeviceListing, DiskInfoError};
use crate::domain::{Device, DeviceKind, PartitionScheme};
use crate::events::RecordingSink;
use crate::runner::CommandFailure;

/// 決めておいた結果を順に返す`DiskInfo`
#[derive(Default)]
struct FakeDiskInfo {
    results: Mutex<Vec<Result<DeviceListing, DiskInfoError>>>,
    calls: AtomicUsize,
}

impl FakeDiskInfo {
    fn with(results: Vec<Result<DeviceListing, DiskInfoError>>) -> Self {
        Self {
            results: Mutex::new(results),
            calls: AtomicUsize::new(0),
        }
    }
}

impl DiskInfo for FakeDiskInfo {
    fn list_devices(&self) -> Result<DeviceListing, DiskInfoError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.results.lock().unwrap().remove(0)
    }
}

struct FakeBootInfo(AtomicUsize);

impl BootInfo for FakeBootInfo {
    fn boot_part_uuid(&self) -> Option<String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Some("6C5DB8D5-6B5C-4C61-ACCB-C7C2F1E2B6B6".into())
    }
}

fn device(id: &str) -> Device {
    Device {
        id: id.into(),
        kind: DeviceKind::External,
        labels: vec![],
        extra_label_count: 0,
        vendor: None,
        image: None,
        connection: None,
        scheme: PartitionScheme::Gpt,
        size: 1,
        partitions: vec![],
    }
}

fn listing(ids: &[&str], failures: &[&str]) -> Result<DeviceListing, DiskInfoError> {
    Ok(DeviceListing {
        devices: ids.iter().map(|id| device(id)).collect(),
        command: "diskutil list -plist".into(),
        failures: failures
            .iter()
            .map(|command| CommandFailure {
                command: (*command).into(),
                stderr: "boom".into(),
            })
            .collect(),
    })
}

struct Setup {
    core: AppCore,
    disk_info: Arc<FakeDiskInfo>,
    boot_info: Arc<FakeBootInfo>,
    sink: Arc<RecordingSink>,
}

fn setup(results: Vec<Result<DeviceListing, DiskInfoError>>) -> Setup {
    let disk_info = Arc::new(FakeDiskInfo::with(results));
    let boot_info = Arc::new(FakeBootInfo(AtomicUsize::new(0)));
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(disk_info.clone(), boot_info.clone(), sink.clone());
    Setup {
        core,
        disk_info,
        boot_info,
        sink,
    }
}

fn log_lines(core: &AppCore) -> Vec<(String, LogResult)> {
    core.get_log()
        .into_iter()
        .map(|e| (e.command, e.result))
        .collect()
}

#[test]
fn list_devices_loads_once_and_logs_one_line() {
    let s = setup(vec![listing(&["disk0", "disk4"], &[])]);
    let first = s.core.list_devices().unwrap();
    let second = s.core.list_devices().unwrap();
    assert_eq!(first, second);
    assert_eq!(s.disk_info.calls.load(Ordering::SeqCst), 1);
    assert_eq!(first.devices.len(), 2);
    assert_eq!(
        first.boot_part_uuid.as_deref(),
        Some("6C5DB8D5-6B5C-4C61-ACCB-C7C2F1E2B6B6")
    );
    assert!(first.taken_at > 0);
    assert_eq!(
        log_lines(&s.core),
        [("diskutil list -plist".to_owned(), LogResult::Ok)]
    );
    // 一覧を返すだけでは、devices-changedは出さない
    assert_eq!(s.sink.names(), ["log-appended"]);
}

#[test]
fn refresh_reloads_and_emits_devices_changed() {
    let s = setup(vec![
        listing(&["disk0"], &[]),
        listing(&["disk0", "disk6"], &[]),
    ]);
    s.core.list_devices().unwrap();
    s.sink.take();
    let snapshot = s.core.refresh().unwrap();
    assert_eq!(snapshot.devices.len(), 2);
    let events = s.sink.take();
    let names: Vec<_> = events.iter().map(AppEvent::name).collect();
    assert_eq!(names, ["log-appended", "devices-changed"]);
    assert_eq!(events[1], AppEvent::DevicesChanged(snapshot.clone()));
    // 取得し直した一覧が、以後のlist_devicesで返る
    assert_eq!(s.core.list_devices().unwrap(), snapshot);
    // 起動に使ったESPは1回だけ読む
    assert_eq!(s.boot_info.0.load(Ordering::SeqCst), 1);
}

#[test]
fn partial_failures_are_logged_as_separate_fail_lines() {
    let s = setup(vec![listing(&["disk0"], &["diskutil info -plist disk4"])]);
    s.core.list_devices().unwrap();
    assert_eq!(
        log_lines(&s.core),
        [
            ("diskutil list -plist".to_owned(), LogResult::Ok),
            ("diskutil info -plist disk4".to_owned(), LogResult::Fail),
        ]
    );
}

#[test]
fn list_failure_is_logged_and_returned() {
    let failure = CommandFailure {
        command: "diskutil list -plist".into(),
        stderr: "denied".into(),
    };
    let s = setup(vec![
        Err(DiskInfoError::CommandFailed(failure)),
        listing(&["disk0"], &[]),
    ]);
    assert_eq!(
        s.core.list_devices().unwrap_err(),
        AppError::CommandFailed {
            command: "diskutil list -plist".into(),
            stderr: "denied".into(),
        }
    );
    assert_eq!(
        log_lines(&s.core),
        [("diskutil list -plist".to_owned(), LogResult::Fail)]
    );
    // 失敗した結果は保持せず、次の呼び出しで取得し直す
    assert!(s.core.list_devices().is_ok());
}

#[test]
fn operations_on_the_same_disk_are_exclusive_and_emit_busy() {
    let s = setup(vec![]);
    let guard = s
        .core
        .begin_operation("disk4s1", Operation::MountEsp)
        .unwrap();
    assert!(matches!(
        s.core.begin_operation("disk4", Operation::EjectDevice),
        Err(AppError::Busy)
    ));
    assert!(matches!(
        s.core.begin_operation("disk4s1; ls", Operation::MountEsp),
        Err(AppError::InvalidTarget)
    ));
    drop(guard);
    let events = s.sink.take();
    assert_eq!(
        events,
        [
            AppEvent::Busy(BusyState {
                target: "disk4s1".into(),
                op: Operation::MountEsp,
                active: true
            }),
            AppEvent::Busy(BusyState {
                target: "disk4s1".into(),
                op: Operation::MountEsp,
                active: false
            }),
        ]
    );
    assert!(
        s.core
            .begin_operation("disk4", Operation::EjectDevice)
            .is_ok()
    );
}

#[test]
fn clear_log_empties_it() {
    let s = setup(vec![listing(&[], &[])]);
    s.core.list_devices().unwrap();
    s.core.clear_log();
    assert!(s.core.get_log().is_empty());
    assert_eq!(s.sink.names(), ["log-appended", "log-cleared"]);
}
