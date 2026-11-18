use std::path::{Path, PathBuf};

use super::*;
use crate::runner::fake::FakeRunner;

/// 実機（Apple siliconの内蔵ディスク）で`scripts/test_image.sh`の3つのイメージを接続し、
/// gptのESPをマウントした状態の出力。UUID・パスは架空の値に置き換えてある
fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/diskutil/images_attached")
}

fn adapter(runner: FakeRunner) -> (DiskutilAdapter, Arc<FakeRunner>) {
    let runner = Arc::new(runner);
    let adapter = DiskutilAdapter::with_mount_source(
        runner.clone(),
        Box::new(|path| path.starts_with("/Users").then(|| "disk3s5".to_owned())),
    );
    (adapter, runner)
}

fn device<'a>(listing: &'a DeviceListing, id: &str) -> &'a Device {
    listing.devices.iter().find(|d| d.id == id).unwrap()
}

#[test]
fn lists_physical_disks_and_images_without_synthesized_disks() {
    let (adapter, _) = adapter(FakeRunner::from_fixture_dir(&fixture_dir()));
    let listing = adapter.list_devices().unwrap();
    let ids: Vec<&str> = listing.devices.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["disk0", "disk4", "disk6", "disk8"]);
    assert_eq!(listing.command, "diskutil list -plist");
    assert!(listing.failures.is_empty(), "{:?}", listing.failures);
}

#[test]
fn internal_disk_uses_root_volume_as_label() {
    let (adapter, _) = adapter(FakeRunner::from_fixture_dir(&fixture_dir()));
    let listing = adapter.list_devices().unwrap();
    let disk0 = device(&listing, "disk0");
    assert_eq!(disk0.kind, DeviceKind::Internal);
    assert_eq!(disk0.vendor.as_deref(), Some("APPLE SSD AP0512Q"));
    assert_eq!(disk0.connection.as_deref(), Some("Apple Fabric"));
    assert_eq!(disk0.scheme, PartitionScheme::Gpt);
    assert_eq!(disk0.size, 500_277_792_768);
    assert_eq!(disk0.image, None);
    // ISC・Recoveryのコンテナは、OSが内部で使うボリュームだけなので候補にならない
    assert_eq!(disk0.labels, ["Macintosh HD"]);
    assert_eq!(disk0.extra_label_count, 0);

    let kinds: Vec<(&str, PartitionKind, &str)> = disk0
        .partitions
        .iter()
        .map(|p| (p.id.as_str(), p.kind, p.type_name.as_str()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("disk0s1", PartitionKind::Apfs, "Apple_APFS_ISC"),
            ("disk0s2", PartitionKind::Apfs, "Apple_APFS"),
            ("disk0s3", PartitionKind::Apfs, "Apple_APFS_Recovery"),
        ]
    );
    let main = &disk0.partitions[1];
    assert_eq!(main.filesystem.as_deref(), Some("APFS"));
    assert_eq!(main.offset, Some(524_312_576));
    let container = main.apfs.as_ref().unwrap();
    assert_eq!(container.id, "disk3");
    // スナップショット（disk3s1s1）は並べず、ボリューム本体のマウント先を`/`にする
    let root = &container.volumes[0];
    assert_eq!(
        (
            root.id.as_str(),
            root.name.as_str(),
            root.mount_point.as_deref()
        ),
        ("disk3s1", "Macintosh HD", Some("/"))
    );
    assert!(container.volumes.iter().all(|v| v.id != "disk3s1s1"));
    assert_eq!(root.used, Some(13_756_141_568));
}

#[test]
fn disk_image_has_path_and_store_and_esp() {
    let (adapter, _) = adapter(FakeRunner::from_fixture_dir(&fixture_dir()));
    let listing = adapter.list_devices().unwrap();
    let disk4 = device(&listing, "disk4");
    assert_eq!(disk4.kind, DeviceKind::Image);
    assert_eq!(disk4.labels, ["gpt.sparseimage"]);
    assert_eq!(disk4.extra_label_count, 0);
    assert_eq!(disk4.vendor, None);
    assert_eq!(disk4.connection.as_deref(), Some("Disk Image"));
    assert_eq!(
        disk4.image,
        Some(DiskImage {
            path: "/Users/alice/Developer/esp_latch/target/test_images/gpt.sparseimage".into(),
            stored_on: Some("disk3s5".into()),
        })
    );

    let esp = &disk4.partitions[0];
    assert_eq!(
        esp,
        &Partition {
            id: "disk4s1".into(),
            kind: PartitionKind::Esp,
            type_name: "EFI".into(),
            filesystem: Some("FAT32".into()),
            name: Some("EFI".into()),
            offset: Some(20480),
            size: 200_003_584,
            part_uuid: Some("00000000-0000-4000-A000-000000000004".into()),
            mount_point: Some("/Volumes/EFI".into()),
            apfs: None,
        }
    );
    let volumes = &disk4.partitions[1].apfs.as_ref().unwrap().volumes;
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0].name, "ESP Latch Test");
    assert_eq!(
        volumes[0].mount_point.as_deref(),
        Some("/Volumes/ESP Latch Test")
    );
}

#[test]
fn unmountable_esp_and_mbr_disk() {
    let (adapter, _) = adapter(FakeRunner::from_fixture_dir(&fixture_dir()));
    let listing = adapter.list_devices().unwrap();

    let broken_esp = &device(&listing, "disk6").partitions[0];
    assert_eq!(broken_esp.kind, PartitionKind::Esp);
    assert_eq!(broken_esp.name, None);
    assert_eq!(broken_esp.mount_point, None);

    let disk8 = device(&listing, "disk8");
    assert_eq!(disk8.scheme, PartitionScheme::Mbr);
    let part = &disk8.partitions[0];
    assert_eq!(part.kind, PartitionKind::Other);
    assert_eq!(part.type_name, "Windows_NTFS");
    assert_eq!(part.filesystem.as_deref(), Some("ExFAT"));
    assert_eq!(part.part_uuid, None);
}

#[test]
fn runs_info_for_each_disk_and_partition_then_hdiutil() {
    let (adapter, runner) = adapter(FakeRunner::from_fixture_dir(&fixture_dir()));
    adapter.list_devices().unwrap();
    let calls = runner.calls();
    assert_eq!(
        calls.first().map(String::as_str),
        Some("diskutil list -plist")
    );
    assert_eq!(
        calls.last().map(String::as_str),
        Some("hdiutil info -plist")
    );
    let mut infos: Vec<&str> = calls[1..calls.len() - 1]
        .iter()
        .map(String::as_str)
        .collect();
    infos.sort_unstable();
    let mut expected: Vec<String> = [
        "disk0", "disk0s1", "disk0s2", "disk0s3", "disk4", "disk4s1", "disk4s2", "disk6",
        "disk6s1", "disk6s2", "disk8", "disk8s1",
    ]
    .iter()
    .map(|id| format!("diskutil info -plist {id}"))
    .collect();
    expected.sort_unstable();
    assert_eq!(infos, expected);
}

#[test]
fn list_failure_is_an_error() {
    let runner = FakeRunner::new();
    runner.respond("diskutil list -plist", 1, b"", "Unable to run because...");
    let (adapter, _) = adapter(runner);
    assert_eq!(
        adapter.list_devices().unwrap_err(),
        DiskInfoError::CommandFailed(CommandFailure {
            command: "diskutil list -plist".into(),
            stderr: "Unable to run because...".into(),
        })
    );
}

#[test]
fn unparsable_list_is_an_error() {
    let runner = FakeRunner::new();
    runner.respond("diskutil list -plist", 0, b"not a plist", "");
    let (adapter, _) = adapter(runner);
    assert!(matches!(
        adapter.list_devices().unwrap_err(),
        DiskInfoError::Parse { command, .. } if command == "diskutil list -plist"
    ));
}

#[test]
fn failed_info_and_hdiutil_are_reported_and_filled_as_unknown() {
    let runner = FakeRunner::from_fixture_dir(&fixture_dir());
    runner.respond(
        "diskutil info -plist disk8",
        1,
        b"",
        "Could not find disk: disk8",
    );
    runner.respond("hdiutil info -plist", 1, b"", "hdiutil: info failed");
    let (adapter, _) = adapter(runner);
    let listing = adapter.list_devices().unwrap();

    let failed: Vec<&str> = listing
        .failures
        .iter()
        .map(|f| f.command.as_str())
        .collect();
    assert_eq!(
        failed,
        ["diskutil info -plist disk8", "hdiutil info -plist"]
    );
    assert_eq!(listing.failures[0].stderr, "Could not find disk: disk8");

    let disk8 = device(&listing, "disk8");
    assert_eq!(disk8.kind, DeviceKind::External);
    assert_eq!(disk8.vendor, None);
    assert_eq!(disk8.connection, None);
    assert_eq!(disk8.labels, ["ESPLATCHMBR"]);
    // イメージのパスは分からないが、接続の種別からディスクイメージと分かる
    let disk4 = device(&listing, "disk4");
    assert_eq!(disk4.kind, DeviceKind::Image);
    assert_eq!(disk4.image, None);
    assert_eq!(disk4.labels, ["ESP Latch Test"]);
}

fn part(id: &str, kind: PartitionKind, name: Option<&str>, mount: Option<&str>) -> Partition {
    Partition {
        id: id.into(),
        kind,
        type_name: String::new(),
        filesystem: None,
        name: name.map(Into::into),
        offset: None,
        size: 1,
        part_uuid: None,
        mount_point: mount.map(Into::into),
        apfs: None,
    }
}

fn volume(id: &str, name: &str, mount: Option<&str>) -> ApfsVolume {
    ApfsVolume {
        id: id.into(),
        name: name.into(),
        mount_point: mount.map(Into::into),
        used: None,
    }
}

fn apfs(id: &str, container: &str, volumes: Vec<ApfsVolume>) -> Partition {
    Partition {
        apfs: Some(ApfsContainer {
            id: container.into(),
            volumes,
        }),
        ..part(id, PartitionKind::Apfs, None, None)
    }
}

#[test]
fn labels_skip_esp_and_unmounted_other_partitions() {
    // Windowsのディスク：ESP・MSR（未マウント）・Windows（マウント中）・回復（未マウント）
    let parts = [
        part("disk4s1", PartitionKind::Esp, Some("SYSTEM"), None),
        part("disk4s2", PartitionKind::Other, None, None),
        part(
            "disk4s3",
            PartitionKind::Other,
            Some("Windows"),
            Some("/Volumes/Windows"),
        ),
        part("disk4s4", PartitionKind::Other, Some("Windows RE"), None),
    ];
    assert_eq!(choose_labels(&parts, &HashSet::new()), ["Windows"]);
}

#[test]
fn labels_include_unmounted_hfs_and_count_one_per_partition() {
    let parts = [
        part("disk2s1", PartitionKind::Esp, Some("EFI"), None),
        part("disk2s2", PartitionKind::Hfs, Some("Install macOS"), None),
        apfs(
            "disk2s3",
            "disk9",
            vec![
                volume("disk9s1", "Backup", None),
                volume("disk9s2", "Data", Some("/Volumes/Data")),
            ],
        ),
    ];
    // APFSは、/がなければ最初のマウント中のボリューム
    assert_eq!(
        choose_labels(&parts, &HashSet::new()),
        ["Install macOS", "Data"]
    );
}

#[test]
fn apfs_label_prefers_root_then_mounted_then_first_and_skips_internal() {
    let root = apfs(
        "disk0s2",
        "disk3",
        vec![
            volume("disk3s5", "Data", Some("/System/Volumes/Data")),
            volume("disk3s1", "Macintosh HD", Some("/")),
        ],
    );
    assert_eq!(choose_labels(&[root], &HashSet::new()), ["Macintosh HD"]);

    let unmounted = apfs(
        "disk5s2",
        "disk6",
        vec![
            volume("disk6s1", "First", None),
            volume("disk6s2", "Second", None),
        ],
    );
    assert_eq!(choose_labels(&[unmounted], &HashSet::new()), ["First"]);

    let internal = apfs(
        "disk0s1",
        "disk1",
        vec![volume("disk1s1", "iSCPreboot", Some("/x"))],
    );
    let internal_ids = HashSet::from(["disk1s1".to_owned()]);
    assert!(choose_labels(&[internal], &internal_ids).is_empty());
}

#[test]
fn recognizes_partition_kinds_and_schemes() {
    assert_eq!(partition_kind("EFI"), PartitionKind::Esp);
    assert_eq!(
        partition_kind("c12a7328-f81f-11d2-ba4b-00a0c93ec93b"),
        PartitionKind::Esp
    );
    assert_eq!(partition_kind("Apple_HFS"), PartitionKind::Hfs);
    assert_eq!(partition_kind("Apple_APFS_Recovery"), PartitionKind::Apfs);
    assert_eq!(partition_kind("Microsoft Basic Data"), PartitionKind::Other);
    assert_eq!(
        partition_scheme(Some("Apple_partition_scheme")),
        PartitionScheme::Apm
    );
    assert_eq!(partition_scheme(Some("Apple_HFS")), PartitionScheme::None);
}

#[test]
fn recognizes_snapshot_and_whole_disk_ids() {
    assert!(is_snapshot_id("disk3s1s1"));
    assert!(!is_snapshot_id("disk3s1"));
    assert!(is_whole_disk_id("disk12"));
    assert!(!is_whole_disk_id("disk12s1"));
    assert!(!is_whole_disk_id("disk"));
}

#[test]
fn statfs_finds_a_device_for_existing_paths() {
    let dir = tempfile::tempdir().unwrap();
    let source = statfs_mount_source(dir.path()).unwrap();
    assert!(source.starts_with("disk"), "{source}");
    assert_eq!(statfs_mount_source(Path::new("/nonexistent/path")), None);
}
