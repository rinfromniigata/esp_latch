//! ESPのユースケース（mount_esp・unmount_esp・open_esp・inspect_esp）

use super::*;
use crate::privileged::{
    CommandRecord, HelperError, Launch, Op, PrivilegedError, PrivilegedErrorKind, PrivilegedReply,
    Response,
};

const BOOT_UUID: &str = "6C5DB8D5-6B5C-4C61-ACCB-C7C2F1E2B6B6";

fn esp(id: &str, mount_point: Option<&str>, part_uuid: Option<&str>) -> PartitionInfo {
    PartitionInfo {
        id: id.into(),
        is_esp: true,
        mount_point: mount_point.map(str::to_owned),
        part_uuid: part_uuid.map(str::to_owned),
    }
}

fn record(command: &str, status: i32, stderr: &str) -> CommandRecord {
    CommandRecord {
        command: command.into(),
        status: Some(status),
        stderr: stderr.into(),
    }
}

fn launch(result: LogResult) -> Launch {
    Launch {
        command: "osascript -e 'do shell script …'".into(),
        result,
    }
}

fn busy_events(sink: &RecordingSink) -> Vec<(String, Operation, bool)> {
    sink.take()
        .into_iter()
        .filter_map(|event| match event {
            AppEvent::Busy(state) => Some((state.target, state.op, state.active)),
            _ => None,
        })
        .collect()
}

#[test]
fn mount_logs_launch_and_commands_then_opens_finder() {
    let s = setup(vec![]);
    s.disk_info
        .set_partition(esp("disk4s1", Some("/Volumes/EFI"), None));
    s.privileged.push(Ok(PrivilegedReply {
        launch: Some(launch(LogResult::Ok)),
        response: Response::success(1, vec![record("diskutil mount disk4s1", 0, "")]),
    }));
    s.runner.respond("open /Volumes/EFI", 0, b"", "");

    let detail = s.core.mount_esp("disk4s1", true).unwrap();
    assert_eq!(detail.part, "disk4s1");
    assert_eq!(detail.mount_point, "/Volumes/EFI");
    assert_eq!(detail.loader.unwrap().name, LoaderName::Opencore);
    assert_eq!(
        s.privileged.received(),
        [Op::MountEsp {
            part: "disk4s1".into()
        }]
    );
    assert_eq!(
        log_lines(&s.core),
        [
            ("osascript -e 'do shell script …'".to_owned(), LogResult::Ok),
            ("diskutil mount disk4s1".to_owned(), LogResult::Ok),
            ("open /Volumes/EFI".to_owned(), LogResult::Ok),
            ("diskutil list -plist".to_owned(), LogResult::Ok),
        ]
    );
    // 一覧を取得し直してから、処理中を終える
    let names = s.sink.names();
    let changed = names.iter().position(|n| *n == "devices-changed").unwrap();
    let last_busy = names.iter().rposition(|n| *n == "busy").unwrap();
    assert!(changed < last_busy, "{names:?}");
    assert_eq!(
        busy_events(&s.sink),
        [
            ("disk4s1".into(), Operation::MountEsp, true),
            ("disk4s1".into(), Operation::MountEsp, false)
        ]
    );
}

#[test]
fn mount_without_finder_and_without_new_launch() {
    let s = setup(vec![]);
    s.disk_info
        .set_partition(esp("disk4s1", Some("/Volumes/EFI 1"), None));
    s.privileged.push_response(Response::success(
        1,
        vec![record("diskutil mount disk4s1", 0, "")],
    ));
    s.core.mount_esp("disk4s1", false).unwrap();
    assert!(s.runner.calls().is_empty(), "Finder is not opened");
    assert_eq!(
        log_lines(&s.core),
        [
            ("diskutil mount disk4s1".to_owned(), LogResult::Ok),
            ("diskutil list -plist".to_owned(), LogResult::Ok),
        ]
    );
}

#[test]
fn finder_failure_does_not_fail_the_mount() {
    let s = setup(vec![]);
    s.disk_info
        .set_partition(esp("disk4s1", Some("/Volumes/EFI"), None));
    s.privileged.push_response(Response::success(1, vec![]));
    s.runner
        .respond("open /Volumes/EFI", 1, b"", "LSOpenURLs failed");
    assert!(s.core.mount_esp("disk4s1", true).is_ok());
    assert!(log_lines(&s.core).contains(&("open /Volumes/EFI".to_owned(), LogResult::Fail)));
}

#[test]
fn cancelled_authentication_is_logged_and_returned() {
    let s = setup(vec![]);
    s.disk_info.set_partition(esp("disk4s1", None, None));
    s.privileged.push(Err(PrivilegedError {
        kind: PrivilegedErrorKind::AuthCancelled,
        launch: Some(launch(LogResult::Cancelled)),
    }));
    assert_eq!(
        s.core.mount_esp("disk4s1", true),
        Err(AppError::AuthCancelled)
    );
    assert_eq!(
        log_lines(&s.core),
        [(
            "osascript -e 'do shell script …'".to_owned(),
            LogResult::Cancelled
        )]
    );
    // 処理中を終え、再試行できる
    assert_eq!(
        busy_events(&s.sink),
        [
            ("disk4s1".into(), Operation::MountEsp, true),
            ("disk4s1".into(), Operation::MountEsp, false)
        ]
    );
    s.disk_info
        .set_partition(esp("disk4s1", Some("/Volumes/EFI"), None));
    s.privileged.push(Ok(PrivilegedReply {
        launch: Some(launch(LogResult::Ok)),
        response: Response::success(2, vec![record("diskutil mount disk4s1", 0, "")]),
    }));
    assert!(s.core.mount_esp("disk4s1", false).is_ok());
}

#[test]
fn unavailable_helper_is_reported() {
    let s = setup(vec![]);
    s.privileged.push(Err(PrivilegedError {
        kind: PrivilegedErrorKind::Unavailable {
            message: "boom".into(),
        },
        launch: Some(launch(LogResult::Fail)),
    }));
    assert_eq!(
        s.core.mount_esp("disk4s1", true),
        Err(AppError::HelperUnavailable)
    );
    assert_eq!(log_lines(&s.core)[0].1, LogResult::Fail);
}

#[test]
fn mount_failure_returns_diskutil_error_and_logs_every_command() {
    let s = setup(vec![]);
    s.privileged.push_response(Response::failure(
        1,
        vec![
            record(
                "diskutil mount disk6s1",
                1,
                "Volume on disk6s1 failed to mount",
            ),
            record(
                "mount -t msdos -o -u=501,-g=20 /dev/disk6s1 /Volumes/EFI",
                71,
                "mount_msdos: /dev/disk6s1: Invalid argument",
            ),
        ],
        HelperError::CommandFailed {
            command: "diskutil mount disk6s1".into(),
            stderr: "Volume on disk6s1 failed to mount".into(),
        },
    ));
    assert_eq!(
        s.core.mount_esp("disk6s1", true),
        Err(AppError::CommandFailed {
            command: "diskutil mount disk6s1".into(),
            stderr: "Volume on disk6s1 failed to mount".into(),
        })
    );
    let lines = log_lines(&s.core);
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|(_, result)| *result == LogResult::Fail));
}

#[test]
fn helper_refusals_map_to_app_errors() {
    for (helper, app) in [
        (HelperError::NotEsp, AppError::NotEsp),
        (HelperError::NotMounted, AppError::NotMounted),
        (HelperError::InvalidTarget, AppError::InvalidTarget),
        (HelperError::InvalidToken, AppError::HelperUnavailable),
        (
            HelperError::Io {
                message: "x".into(),
            },
            AppError::Io {
                message: "x".into(),
            },
        ),
    ] {
        let s = setup(vec![]);
        s.privileged
            .push_response(Response::failure(1, vec![], helper));
        assert_eq!(s.core.unmount_esp("disk4s1", false), Err(app));
    }
}

#[test]
fn invalid_or_busy_targets_are_refused_before_the_helper() {
    let s = setup(vec![]);
    for part in ["disk4", "/dev/disk4s1", "disk4s1; reboot", ""] {
        assert_eq!(
            s.core.mount_esp(part, true),
            Err(AppError::InvalidTarget),
            "{part}"
        );
        assert_eq!(
            s.core.unmount_esp(part, false),
            Err(AppError::InvalidTarget)
        );
        assert_eq!(s.core.open_esp(part), Err(AppError::InvalidTarget));
        assert_eq!(s.core.inspect_esp(part), Err(AppError::InvalidTarget));
    }
    let _eject = s
        .core
        .begin_operation("disk4", Operation::EjectDevice)
        .unwrap();
    assert_eq!(s.core.mount_esp("disk4s1", true), Err(AppError::Busy));
    assert_eq!(s.core.unmount_esp("disk4s2", false), Err(AppError::Busy));
    assert!(s.privileged.received().is_empty());
}

#[test]
fn unmount_logs_and_refreshes_even_on_failure() {
    let s = setup(vec![]);
    s.privileged.push_response(Response::success(
        1,
        vec![record("diskutil unmount disk4s1", 0, "")],
    ));
    s.core.unmount_esp("disk4s1", false).unwrap();
    s.privileged.push_response(Response::failure(
        2,
        vec![record("diskutil unmount disk4s1", 1, "Resource busy")],
        HelperError::CommandFailed {
            command: "diskutil unmount disk4s1".into(),
            stderr: "Resource busy".into(),
        },
    ));
    assert!(matches!(
        s.core.unmount_esp("disk4s1", false),
        Err(AppError::CommandFailed { .. })
    ));
    assert_eq!(
        log_lines(&s.core),
        [
            ("diskutil unmount disk4s1".to_owned(), LogResult::Ok),
            ("diskutil list -plist".to_owned(), LogResult::Ok),
            ("diskutil unmount disk4s1".to_owned(), LogResult::Fail),
            ("diskutil list -plist".to_owned(), LogResult::Ok),
        ]
    );
}

#[test]
fn open_esp_reads_the_mount_point_again() {
    let s = setup(vec![]);
    s.disk_info
        .set_partition(esp("disk4s1", Some("/Volumes/EFI 1"), None));
    s.runner.respond("open '/Volumes/EFI 1'", 0, b"", "");
    s.core.open_esp("disk4s1").unwrap();
    assert_eq!(
        log_lines(&s.core),
        [("open '/Volumes/EFI 1'".to_owned(), LogResult::Ok)]
    );
    assert!(s.privileged.received().is_empty(), "no privileges needed");

    s.disk_info.set_partition(esp("disk4s1", None, None));
    assert_eq!(s.core.open_esp("disk4s1"), Err(AppError::NotMounted));

    s.disk_info
        .set_partition(esp("disk4s1", Some("/Volumes/EFI"), None));
    s.runner.respond("open /Volumes/EFI", 1, b"", "failed");
    assert_eq!(
        s.core.open_esp("disk4s1"),
        Err(AppError::CommandFailed {
            command: "open /Volumes/EFI".into(),
            stderr: "failed".into()
        })
    );
}

#[test]
fn inspect_checks_the_partition() {
    let s = setup(vec![]);
    assert_eq!(s.core.inspect_esp("disk9s1"), Err(AppError::InvalidTarget));
    s.disk_info.set_partition(PartitionInfo {
        is_esp: false,
        ..esp("disk0s2", Some("/"), None)
    });
    assert_eq!(s.core.inspect_esp("disk0s2"), Err(AppError::NotEsp));
    s.disk_info.set_partition(esp("disk4s1", None, None));
    assert_eq!(s.core.inspect_esp("disk4s1"), Err(AppError::NotMounted));
}

#[test]
fn opencore_version_from_nvram_only_for_the_boot_esp() {
    let s = setup(vec![]);
    s.disk_info
        .set_partition(esp("disk0s1", Some("/Volumes/EFI"), Some(BOOT_UUID)));
    s.disk_info.set_partition(esp(
        "disk4s1",
        Some("/Volumes/EFI 1"),
        Some("0A1B2C3D-4E5F-6071-8293-A4B5C6D7E8F9"),
    ));
    let boot = s.core.inspect_esp("disk0s1").unwrap();
    assert_eq!(boot.loader.unwrap().version.as_deref(), Some("1.0.8"));
    let other = s.core.inspect_esp("disk4s1").unwrap();
    assert_eq!(other.loader.unwrap().version, None);
    assert_eq!(
        *s.inspector.opencore_hints.lock().unwrap(),
        [Some("REL-108-2026-09-28".to_owned()), None]
    );
    assert_eq!(other.usage.free, Some(160_000_000));
    // 読むだけの操作は、操作ログに残さない
    assert!(log_lines(&s.core).is_empty());
}

fn trash_record(mount_point: &str) -> CommandRecord {
    record(&format!("rm -rf {mount_point}/.Trashes"), 0, "")
}

fn emptied(id: u64, freed: u64) -> Response {
    Response {
        freed_bytes: Some(freed),
        ..Response::success(id, vec![trash_record("/Volumes/EFI")])
    }
}

#[test]
fn empty_trash_returns_the_freed_bytes_and_refreshes() {
    let s = setup(vec![]);
    s.privileged.push_response(emptied(1, 158_564_352));
    assert_eq!(s.core.empty_esp_trash("disk4s1"), Ok(158_564_352));
    assert_eq!(
        s.privileged.received(),
        [Op::EmptyTrash {
            part: "disk4s1".into()
        }]
    );
    assert_eq!(
        log_lines(&s.core),
        [
            ("rm -rf /Volumes/EFI/.Trashes".to_owned(), LogResult::Ok),
            ("diskutil list -plist".to_owned(), LogResult::Ok),
        ]
    );
    assert!(s.sink.names().contains(&"devices-changed"));
    assert_eq!(
        busy_events(&s.sink),
        [
            ("disk4s1".into(), Operation::EmptyEspTrash, true),
            ("disk4s1".into(), Operation::EmptyEspTrash, false)
        ]
    );
}

#[test]
fn empty_trash_without_a_reported_size_frees_zero() {
    let s = setup(vec![]);
    s.privileged
        .push_response(Response::success(1, vec![trash_record("/Volumes/EFI")]));
    assert_eq!(s.core.empty_esp_trash("disk4s1"), Ok(0));
}

#[test]
fn empty_trash_failures_are_returned_and_logged() {
    let s = setup(vec![]);
    s.privileged.push_response(Response::failure(
        1,
        vec![record(
            "rm -rf /Volumes/EFI/.Trashes",
            1,
            "Operation not permitted",
        )],
        HelperError::Io {
            message: "Operation not permitted".into(),
        },
    ));
    assert_eq!(
        s.core.empty_esp_trash("disk4s1"),
        Err(AppError::Io {
            message: "Operation not permitted".into()
        })
    );
    assert_eq!(log_lines(&s.core)[0].1, LogResult::Fail);
    s.privileged
        .push_response(Response::failure(2, vec![], HelperError::NotMounted));
    assert_eq!(s.core.empty_esp_trash("disk4s1"), Err(AppError::NotMounted));
}

#[test]
fn empty_trash_validates_and_locks_the_target() {
    let s = setup(vec![]);
    for part in ["disk4", "../disk4s1", "disk4s1 ", ""] {
        assert_eq!(s.core.empty_esp_trash(part), Err(AppError::InvalidTarget));
    }
    let _mounting = s
        .core
        .begin_operation("disk4s1", Operation::MountEsp)
        .unwrap();
    assert_eq!(s.core.empty_esp_trash("disk4s1"), Err(AppError::Busy));
    assert!(s.privileged.received().is_empty());
}

#[test]
fn unmount_with_empty_trash_empties_first_then_unmounts() {
    let s = setup(vec![]);
    s.privileged.push(Ok(PrivilegedReply {
        launch: Some(launch(LogResult::Ok)),
        response: emptied(1, 1000),
    }));
    s.privileged.push_response(Response::success(
        2,
        vec![record("diskutil unmount disk4s1", 0, "")],
    ));
    s.core.unmount_esp("disk4s1", true).unwrap();
    assert_eq!(
        s.privileged.received(),
        [
            Op::EmptyTrash {
                part: "disk4s1".into()
            },
            Op::UnmountEsp {
                part: "disk4s1".into()
            }
        ]
    );
    assert_eq!(
        log_lines(&s.core),
        [
            ("osascript -e 'do shell script …'".to_owned(), LogResult::Ok),
            ("rm -rf /Volumes/EFI/.Trashes".to_owned(), LogResult::Ok),
            ("diskutil unmount disk4s1".to_owned(), LogResult::Ok),
            ("diskutil list -plist".to_owned(), LogResult::Ok),
        ]
    );
    // 取り外しは1つの操作として、処理中の通知は1回だけ
    assert_eq!(
        busy_events(&s.sink),
        [
            ("disk4s1".into(), Operation::UnmountEsp, true),
            ("disk4s1".into(), Operation::UnmountEsp, false)
        ]
    );
}

#[test]
fn unmount_without_empty_trash_does_not_touch_the_trash() {
    let s = setup(vec![]);
    s.privileged.push_response(Response::success(
        1,
        vec![record("diskutil unmount disk4s1", 0, "")],
    ));
    s.core.unmount_esp("disk4s1", false).unwrap();
    assert_eq!(
        s.privileged.received(),
        [Op::UnmountEsp {
            part: "disk4s1".into()
        }]
    );
}

#[test]
fn unmount_is_skipped_when_emptying_the_trash_fails() {
    let s = setup(vec![]);
    s.privileged.push_response(Response::failure(
        1,
        vec![record(
            "rm -rf /Volumes/EFI/.Trashes",
            1,
            "Read-only file system",
        )],
        HelperError::Io {
            message: "Read-only file system".into(),
        },
    ));
    assert_eq!(
        s.core.unmount_esp("disk4s1", true),
        Err(AppError::Io {
            message: "Read-only file system".into()
        })
    );
    assert_eq!(
        s.privileged.received(),
        [Op::EmptyTrash {
            part: "disk4s1".into()
        }],
        "the ESP stays mounted so that the user can retry"
    );
    assert!(s.sink.names().contains(&"devices-changed"));
}

#[test]
fn eject_logs_every_command_and_refreshes() {
    let s = setup(vec![]);
    s.privileged.push(Ok(PrivilegedReply {
        launch: Some(launch(LogResult::Ok)),
        response: Response::success(1, vec![record("diskutil eject disk4", 0, "")]),
    }));
    s.core.eject_device("disk4").unwrap();
    assert_eq!(
        s.privileged.received(),
        [Op::EjectDisk {
            disk: "disk4".into()
        }]
    );
    assert_eq!(
        log_lines(&s.core),
        [
            ("osascript -e 'do shell script …'".to_owned(), LogResult::Ok),
            ("diskutil eject disk4".to_owned(), LogResult::Ok),
            ("diskutil list -plist".to_owned(), LogResult::Ok),
        ]
    );
    assert_eq!(
        busy_events(&s.sink),
        [
            ("disk4".into(), Operation::EjectDevice, true),
            ("disk4".into(), Operation::EjectDevice, false)
        ]
    );
}

#[test]
fn eject_failure_returns_the_diskutil_error() {
    let s = setup(vec![]);
    s.privileged.push_response(Response::failure(
        1,
        vec![record(
            "diskutil eject disk4",
            1,
            "Volume disk4s2 failed to unmount",
        )],
        HelperError::CommandFailed {
            command: "diskutil eject disk4".into(),
            stderr: "Volume disk4s2 failed to unmount".into(),
        },
    ));
    assert_eq!(
        s.core.eject_device("disk4"),
        Err(AppError::CommandFailed {
            command: "diskutil eject disk4".into(),
            stderr: "Volume disk4s2 failed to unmount".into()
        })
    );
    assert_eq!(log_lines(&s.core)[0].1, LogResult::Fail);
}

#[test]
fn eject_validates_and_locks_the_target() {
    let s = setup(vec![]);
    for disk in ["disk4s1", "/dev/disk4", "disk4;ls", ""] {
        assert_eq!(s.core.eject_device(disk), Err(AppError::InvalidTarget));
    }
    // 取り出しは、そのディスクの区画への操作中にも始められない
    let _unmounting = s
        .core
        .begin_operation("disk4s1", Operation::UnmountEsp)
        .unwrap();
    assert_eq!(s.core.eject_device("disk4"), Err(AppError::Busy));
    assert!(s.privileged.received().is_empty());
}

#[test]
fn eject_reports_cancelled_authentication() {
    let s = setup(vec![]);
    s.privileged.push(Err(PrivilegedError {
        kind: PrivilegedErrorKind::AuthCancelled,
        launch: Some(launch(LogResult::Cancelled)),
    }));
    assert_eq!(s.core.eject_device("disk4"), Err(AppError::AuthCancelled));
}
