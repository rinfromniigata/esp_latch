use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::disk_info::{DeviceListing, DiskInfoError, PartitionInfo};
use crate::domain::{BootLoader, Device, DeviceKind, EspUsage, LoaderName, PartitionScheme};
use crate::esp::EspInspect;
use crate::events::RecordingSink;
use crate::privileged::fake::FakePrivilegedOps;
use crate::runner::CommandFailure;
use crate::runner::fake::FakeRunner;

mod esp;

/// 決めておいた結果を順に返す`DiskInfo`
/// 一覧の結果は決めておいたものを順に返し（尽きたら空の一覧）、パーティションの情報は表から返す
#[derive(Default)]
struct FakeDiskInfo {
    results: Mutex<Vec<Result<DeviceListing, DiskInfoError>>>,
    calls: AtomicUsize,
    partitions: Mutex<HashMap<String, PartitionInfo>>,
}

impl FakeDiskInfo {
    fn with(results: Vec<Result<DeviceListing, DiskInfoError>>) -> Self {
        Self {
            results: Mutex::new(results),
            ..Self::default()
        }
    }

    fn set_partition(&self, info: PartitionInfo) {
        self.partitions
            .lock()
            .unwrap()
            .insert(info.id.clone(), info);
    }
}

impl DiskInfo for FakeDiskInfo {
    fn list_devices(&self) -> Result<DeviceListing, DiskInfoError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut results = self.results.lock().unwrap();
        if results.is_empty() {
            listing(&[], &[])
        } else {
            results.remove(0)
        }
    }

    fn partition(&self, part: &str) -> Result<PartitionInfo, DiskInfoError> {
        self.partitions
            .lock()
            .unwrap()
            .get(part)
            .cloned()
            .ok_or_else(|| {
                DiskInfoError::CommandFailed(CommandFailure {
                    command: format!("diskutil info -plist {part}"),
                    stderr: "Could not find disk".into(),
                })
            })
    }
}

/// マウント先ごとに、ブートローダーと、受け取ったOpenCoreの版を記録して返す
#[derive(Default)]
struct FakeInspector {
    opencore_hints: Mutex<Vec<Option<String>>>,
}

impl EspInspect for FakeInspector {
    fn loader(&self, _: &Path, opencore_version: Option<&str>) -> Option<BootLoader> {
        self.opencore_hints
            .lock()
            .unwrap()
            .push(opencore_version.map(str::to_owned));
        Some(BootLoader {
            name: LoaderName::Opencore,
            version: opencore_version.map(|_| "1.0.8".to_owned()),
        })
    }

    fn usage(&self, _: &Path) -> EspUsage {
        EspUsage {
            total: Some(209_715_200),
            free: Some(160_000_000),
            trash: Some(0),
        }
    }
}

struct FakeBootInfo(AtomicUsize);

impl BootInfo for FakeBootInfo {
    fn boot_part_uuid(&self) -> Option<String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Some("6C5DB8D5-6B5C-4C61-ACCB-C7C2F1E2B6B6".into())
    }

    fn opencore_version(&self) -> Option<String> {
        Some("REL-108-2026-09-28".into())
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
    privileged: Arc<FakePrivilegedOps>,
    inspector: Arc<FakeInspector>,
    runner: Arc<FakeRunner>,
    sink: Arc<RecordingSink>,
}

fn setup(results: Vec<Result<DeviceListing, DiskInfoError>>) -> Setup {
    let disk_info = Arc::new(FakeDiskInfo::with(results));
    let boot_info = Arc::new(FakeBootInfo(AtomicUsize::new(0)));
    let privileged = Arc::new(FakePrivilegedOps::default());
    let inspector = Arc::new(FakeInspector::default());
    let runner = Arc::new(FakeRunner::new());
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(
        Ports {
            disk_info: disk_info.clone(),
            boot_info: boot_info.clone(),
            privileged: privileged.clone(),
            inspector: inspector.clone(),
            runner: runner.clone(),
        },
        sink.clone(),
    );
    Setup {
        core,
        disk_info,
        boot_info,
        privileged,
        inspector,
        runner,
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
