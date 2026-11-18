use std::collections::VecDeque;
use std::fs;
use std::os::unix::fs::symlink;

use super::*;
use crate::runner::fake::FakeRunner;

const PEER: Owner = Owner { uid: 501, gid: 20 };

/// `diskutil info -plist <id>`の出力（検証に使う項目だけ）
fn info_plist(id: &str, content: &str, mount_point: Option<&Path>) -> Vec<u8> {
    let mut dict = plist::Dictionary::new();
    dict.insert("DeviceIdentifier".into(), id.into());
    dict.insert("Content".into(), content.into());
    dict.insert(
        "MountPoint".into(),
        mount_point
            .map_or(String::new(), |p| p.to_string_lossy().into_owned())
            .into(),
    );
    let mut out = Vec::new();
    plist::to_writer_xml(&mut out, &plist::Value::Dictionary(dict)).unwrap();
    out
}

/// `statfs`の偽実装。マウント先ごとのマウント元と、空き容量（呼ばれるたびに次の値）を返す
#[derive(Default)]
struct FakeVolumes {
    mounts: Mutex<Vec<(PathBuf, String)>>,
    free: Mutex<VecDeque<u64>>,
}

impl FakeVolumes {
    fn mount(&self, on: &Path, from: &str) {
        self.mounts
            .lock()
            .unwrap()
            .push((on.to_path_buf(), from.to_owned()));
    }

    fn unmount_all(&self) {
        self.mounts.lock().unwrap().clear();
    }
}

impl VolumeFs for FakeVolumes {
    fn mount_info(&self, path: &Path) -> io::Result<MountInfo> {
        let mounts = self.mounts.lock().unwrap();
        let (on, from) = mounts
            .iter()
            .find(|(on, _)| on == path)
            .cloned()
            // マウントポイントでなければ、上位のボリューム（/）の情報を返す
            .unwrap_or_else(|| (PathBuf::from("/"), "/dev/disk3s1s1".into()));
        let free = self.free.lock().unwrap().pop_front().unwrap_or(0);
        Ok(MountInfo { from, on, free })
    }
}

struct Fixture {
    runner: Arc<FakeRunner>,
    volumes: Arc<FakeVolumes>,
    volumes_dir: tempfile::TempDir,
    executor: Executor,
}

fn fixture() -> Fixture {
    let runner = Arc::new(FakeRunner::default());
    let volumes = Arc::new(FakeVolumes::default());
    let volumes_dir = tempfile::tempdir().unwrap();
    let executor = Executor::new(
        runner.clone(),
        volumes.clone(),
        volumes_dir.path().to_path_buf(),
    );
    Fixture {
        runner,
        volumes,
        volumes_dir,
        executor,
    }
}

fn mount(part: &str) -> Op {
    Op::MountEsp { part: part.into() }
}

fn commands(outcome: &Outcome) -> Vec<&str> {
    outcome
        .commands
        .iter()
        .map(|c| c.command.as_str())
        .collect()
}

#[test]
fn mounts_with_diskutil() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", None),
        "",
    );
    f.runner.respond("diskutil mount disk4s1", 0, b"", "");
    let outcome = f.executor.execute(&mount("disk4s1"), PEER);
    assert_eq!(outcome.result, Ok(()));
    assert_eq!(commands(&outcome), ["diskutil mount disk4s1"]);
}

#[test]
fn mount_of_a_mounted_esp_runs_nothing() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", Some(Path::new("/Volumes/EFI"))),
        "",
    );
    let outcome = f.executor.execute(&mount("disk4s1"), PEER);
    assert_eq!(outcome.result, Ok(()));
    assert!(outcome.commands.is_empty());
}

#[test]
fn falls_back_to_mount_msdos() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", None),
        "",
    );
    f.runner.respond(
        "diskutil mount disk4s1",
        1,
        b"",
        "Volume on disk4s1 failed to mount",
    );
    // EFIは使われているので、EFI 1を作る
    fs::create_dir(f.volumes_dir.path().join("EFI")).unwrap();
    let target = f.volumes_dir.path().join("EFI 1");
    let mount_command = display_command(
        "/sbin/mount",
        &[
            "-t",
            "msdos",
            "-o",
            "-u=501,-g=20",
            "/dev/disk4s1",
            target.to_str().unwrap(),
        ],
    );
    f.runner.respond(&mount_command, 0, b"", "");
    let outcome = f.executor.execute(&mount("disk4s1"), PEER);
    assert_eq!(outcome.result, Ok(()));
    assert_eq!(
        commands(&outcome),
        ["diskutil mount disk4s1", mount_command.as_str()]
    );
    assert!(target.is_dir());

    // 取り外したら、作ったマウントポイントを消す
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", Some(&target)),
        "",
    );
    f.runner.respond("diskutil unmount disk4s1", 0, b"", "");
    let outcome = f.executor.execute(
        &Op::UnmountEsp {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert_eq!(outcome.result, Ok(()));
    assert!(!target.exists());
    assert!(
        f.volumes_dir.path().join("EFI").exists(),
        "pre-existing dirs stay"
    );
}

#[test]
fn failed_fallback_reports_diskutil_error_and_removes_mount_point() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk6s1",
        0,
        &info_plist("disk6s1", "EFI", None),
        "",
    );
    f.runner.respond(
        "diskutil mount disk6s1",
        1,
        b"",
        "Volume on disk6s1 failed to mount",
    );
    let outcome = f.executor.execute(&mount("disk6s1"), PEER);
    assert_eq!(
        outcome.result,
        Err(HelperError::CommandFailed {
            command: "diskutil mount disk6s1".into(),
            stderr: "Volume on disk6s1 failed to mount".into(),
        })
    );
    assert_eq!(outcome.commands.len(), 2);
    assert!(outcome.commands[1].command.starts_with("mount -t msdos"));
    assert_eq!(fs::read_dir(f.volumes_dir.path()).unwrap().count(), 0);
}

#[test]
fn refuses_partitions_that_are_not_esp() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk0s2",
        0,
        &info_plist("disk0s2", "Apple_APFS", None),
        "",
    );
    for op in [
        mount("disk0s2"),
        Op::UnmountEsp {
            part: "disk0s2".into(),
        },
        Op::EmptyTrash {
            part: "disk0s2".into(),
        },
    ] {
        let outcome = f.executor.execute(&op, PEER);
        assert_eq!(outcome.result, Err(HelperError::NotEsp), "{op:?}");
        assert!(outcome.commands.is_empty());
    }
    assert_eq!(
        f.runner.calls(),
        ["diskutil info -plist disk0s2"; 3],
        "nothing but the info lookup runs"
    );
}

#[test]
fn refuses_unknown_or_mismatched_partitions() {
    let f = fixture();
    // diskutil infoが失敗する（存在しない）
    let outcome = f.executor.execute(&mount("disk9s1"), PEER);
    assert_eq!(outcome.result, Err(HelperError::InvalidTarget));
    // 別のパーティションの情報が返る
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk5s1", "EFI", None),
        "",
    );
    let outcome = f.executor.execute(&mount("disk4s1"), PEER);
    assert_eq!(outcome.result, Err(HelperError::InvalidTarget));
    // plistとして読めない
    f.runner
        .respond("diskutil info -plist disk7s1", 0, b"not a plist", "");
    let outcome = f.executor.execute(&mount("disk7s1"), PEER);
    assert_eq!(outcome.result, Err(HelperError::InvalidTarget));
}

#[test]
fn refuses_malformed_bsd_names_without_running_anything() {
    let f = fixture();
    for op in [
        mount("disk4"),
        mount("disk4s1; rm -rf /"),
        Op::EmptyTrash {
            part: "../../etc".into(),
        },
        Op::EjectDisk {
            disk: "disk4s1".into(),
        },
        Op::UnmountEsp {
            part: "/dev/disk4s1".into(),
        },
    ] {
        let outcome = f.executor.execute(&op, PEER);
        assert_eq!(outcome.result, Err(HelperError::InvalidTarget), "{op:?}");
    }
    assert!(f.runner.calls().is_empty());
}

#[test]
fn unmount_requires_a_mounted_esp() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", None),
        "",
    );
    let outcome = f.executor.execute(
        &Op::UnmountEsp {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert_eq!(outcome.result, Err(HelperError::NotMounted));
}

#[test]
fn unmount_failure_is_reported() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", Some(Path::new("/Volumes/EFI"))),
        "",
    );
    f.runner
        .respond("diskutil unmount disk4s1", 1, b"", "Resource busy");
    let outcome = f.executor.execute(
        &Op::UnmountEsp {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert_eq!(
        outcome.result,
        Err(HelperError::CommandFailed {
            command: "diskutil unmount disk4s1".into(),
            stderr: "Resource busy".into()
        })
    );
}

#[test]
fn ejects_disks() {
    let f = fixture();
    f.runner.respond("diskutil eject disk4", 0, b"", "");
    let outcome = f.executor.execute(
        &Op::EjectDisk {
            disk: "disk4".into(),
        },
        PEER,
    );
    assert_eq!(outcome.result, Ok(()));
    assert_eq!(commands(&outcome), ["diskutil eject disk4"]);
}

/// ESPのマウント先に見立てたディレクトリ（ゴミ箱つき）
fn esp_volume(f: &Fixture) -> tempfile::TempDir {
    let volume = tempfile::tempdir().unwrap();
    fs::create_dir_all(volume.path().join("EFI/OC")).unwrap();
    fs::write(volume.path().join("EFI/OC/OpenCore.efi"), b"oc").unwrap();
    fs::create_dir_all(volume.path().join(".Trashes/501")).unwrap();
    fs::write(volume.path().join(".Trashes/501/old.efi"), vec![0; 1024]).unwrap();
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", Some(volume.path())),
        "",
    );
    volume
}

#[test]
fn empties_trash_and_reports_freed_bytes() {
    let f = fixture();
    let volume = esp_volume(&f);
    f.volumes.mount(volume.path(), "/dev/disk4s1");
    f.volumes.free.lock().unwrap().extend([10_000, 14_096]);
    let outcome = f.executor.execute(
        &Op::EmptyTrash {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert_eq!(outcome.result, Ok(()));
    assert_eq!(outcome.freed_bytes, Some(4_096));
    let expected = display_command(
        "rm",
        &["-rf", volume.path().join(".Trashes").to_str().unwrap()],
    );
    assert_eq!(commands(&outcome), [expected.as_str()]);
    assert!(volume.path().join("EFI/OC/OpenCore.efi").exists());
    assert_eq!(
        fs::read_dir(volume.path().join(".Trashes"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn empty_trash_checks_the_mount_source() {
    let f = fixture();
    let volume = esp_volume(&f);
    // マウント先が別のパーティションのもの
    f.volumes.mount(volume.path(), "/dev/disk9s1");
    let outcome = f.executor.execute(
        &Op::EmptyTrash {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert_eq!(outcome.result, Err(HelperError::InvalidTarget));
    assert!(volume.path().join(".Trashes/501/old.efi").exists());

    // マウントポイントではない（上位のボリュームの情報が返る）
    f.volumes.unmount_all();
    let outcome = f.executor.execute(
        &Op::EmptyTrash {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert_eq!(outcome.result, Err(HelperError::InvalidTarget));
    assert!(volume.path().join(".Trashes/501/old.efi").exists());
}

#[test]
fn empty_trash_refuses_symlinked_trashes() {
    let f = fixture();
    let volume = esp_volume(&f);
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("precious"), b"keep").unwrap();
    fs::remove_dir_all(volume.path().join(".Trashes")).unwrap();
    symlink(outside.path(), volume.path().join(".Trashes")).unwrap();
    f.volumes.mount(volume.path(), "/dev/disk4s1");
    let outcome = f.executor.execute(
        &Op::EmptyTrash {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert!(matches!(outcome.result, Err(HelperError::Io { .. })));
    assert_eq!(outcome.commands[0].status, Some(1));
    assert!(outside.path().join("precious").exists());
}

#[test]
fn empty_trash_requires_a_mounted_esp() {
    let f = fixture();
    f.runner.respond(
        "diskutil info -plist disk4s1",
        0,
        &info_plist("disk4s1", "EFI", None),
        "",
    );
    let outcome = f.executor.execute(
        &Op::EmptyTrash {
            part: "disk4s1".into(),
        },
        PEER,
    );
    assert_eq!(outcome.result, Err(HelperError::NotMounted));
}

#[test]
fn statfs_reports_the_root_volume() {
    let info = StatfsVolumeFs.mount_info(Path::new("/")).unwrap();
    assert_eq!(info.on, PathBuf::from("/"));
    assert!(info.from.starts_with("/dev/disk"));
}
