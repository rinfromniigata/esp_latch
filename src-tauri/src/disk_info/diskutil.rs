//! `diskutil list -plist`・`diskutil info -plist`・`hdiutil info -plist`から、デバイスの一覧を組み立てる。
//!
//! - `diskutil list`で、ディスク・パーティション・APFSコンテナとボリュームの構成を得る
//! - 各ディスクとパーティションの`diskutil info`で、接続の種別・メディア名・ファイルシステム・位置を得る。
//!   1回に0.1〜0.2秒かかるため、並列に実行する
//! - `hdiutil info`で、ディスクイメージのファイルのパスを得る（ディスクイメージがあるときだけ実行する）
//!
//! `diskutil apfs list`は使わない。コンテナのボリューム・マウント先・使用量は`diskutil list`に含まれるため。

use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use super::raw::{HdiutilInfo, Info, List, ListDisk, ListVolume, non_empty};
use super::{DeviceListing, DiskInfo, DiskInfoError};
use crate::domain::{
    ApfsContainer, ApfsVolume, Device, DeviceKind, DiskImage, Partition, PartitionKind,
    PartitionScheme,
};
use crate::runner::{CommandFailure, CommandRunner, display_command, programs};

/// ESPのGPTの種別GUID
const ESP_TYPE_GUID: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
/// `diskutil info`の`BusProtocol`で、ディスクイメージを表す値
const BUS_DISK_IMAGE: &str = "Disk Image";
/// `diskutil info`を同時に実行する数
const INFO_PARALLELISM: usize = 8;

/// ファイルのパスから、それを保存しているボリュームのBSD名（`disk1s1`）を返す
pub type MountSource = Box<dyn Fn(&Path) -> Option<String> + Send + Sync>;

pub struct DiskutilAdapter {
    runner: Arc<dyn CommandRunner>,
    mount_source: MountSource,
}

impl DiskutilAdapter {
    /// イメージファイルの保存先は`statfs`で調べる
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self::with_mount_source(runner, Box::new(statfs_mount_source))
    }

    pub fn with_mount_source(runner: Arc<dyn CommandRunner>, mount_source: MountSource) -> Self {
        Self {
            runner,
            mount_source,
        }
    }

    fn run_plist<T: DeserializeOwned>(
        &self,
        program: &str,
        args: &[&str],
    ) -> Result<T, PlistError> {
        let output = self
            .runner
            .run_checked(program, args)
            .map_err(PlistError::Command)?;
        plist::from_bytes(&output.stdout).map_err(|error| PlistError::Parse {
            command: display_command(program, args),
            message: error.to_string(),
        })
    }

    /// 各ディスク・パーティションの`diskutil info`を並列に実行する。結果は`ids`と同じ順
    fn run_infos(&self, ids: &[&str]) -> Vec<Result<Info, CommandFailure>> {
        let workers = INFO_PARALLELISM.min(ids.len()).max(1);
        let mut results: Vec<Option<Result<Info, CommandFailure>>> =
            std::iter::repeat_with(|| None).take(ids.len()).collect();
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|worker| {
                    scope.spawn(move || {
                        (worker..ids.len())
                            .step_by(workers)
                            .map(|index| {
                                let result = self
                                    .run_plist::<Info>(
                                        programs::DISKUTIL,
                                        &["info", "-plist", ids[index]],
                                    )
                                    .map_err(PlistError::into_failure);
                                (index, result)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            for handle in handles {
                for (index, result) in handle.join().expect("diskutil info worker panicked") {
                    results[index] = Some(result);
                }
            }
        });
        results
            .into_iter()
            .map(|r| r.expect("every id has a result"))
            .collect()
    }
}

impl DiskInfo for DiskutilAdapter {
    fn list_devices(&self) -> Result<DeviceListing, DiskInfoError> {
        let list_args = ["list", "-plist"];
        let command = display_command(programs::DISKUTIL, &list_args);
        let list: List = self
            .run_plist(programs::DISKUTIL, &list_args)
            .map_err(PlistError::into_disk_info_error)?;

        let disks_by_id: HashMap<&str, &ListDisk> = list
            .all_disks_and_partitions
            .iter()
            .map(|disk| (disk.device_identifier.as_str(), disk))
            .collect();
        // サイドバーに出すのは物理ディスクとディスクイメージだけ（APFSコンテナの合成ディスクを除く）
        let whole_disks: Vec<&ListDisk> = list
            .whole_disks
            .iter()
            .filter_map(|id| disks_by_id.get(id.as_str()).copied())
            .filter(|disk| !disk.is_apfs_container())
            .collect();
        // APFSの物理ストア（パーティション）→ コンテナ
        let containers: HashMap<&str, &ListDisk> = list
            .all_disks_and_partitions
            .iter()
            .filter(|disk| disk.is_apfs_container())
            .flat_map(|disk| {
                disk.apfs_physical_stores
                    .iter()
                    .map(move |store| (store.device_identifier.as_str(), disk))
            })
            .collect();

        let mut failures = Vec::new();
        let ids: Vec<&str> = whole_disks
            .iter()
            .flat_map(|disk| {
                std::iter::once(disk.device_identifier.as_str()).chain(
                    disk.partitions
                        .iter()
                        .map(|part| part.device_identifier.as_str()),
                )
            })
            .collect();
        let infos: HashMap<&str, Info> = ids
            .iter()
            .copied()
            .zip(self.run_infos(&ids))
            .filter_map(|(id, result)| match result {
                Ok(info) => Some((id, info)),
                Err(failure) => {
                    failures.push(failure);
                    None
                }
            })
            .collect();

        let has_image = whole_disks.iter().any(|disk| {
            infos
                .get(disk.device_identifier.as_str())
                .and_then(|info| info.bus_protocol.as_deref())
                == Some(BUS_DISK_IMAGE)
        });
        let images = if has_image {
            match self.run_plist::<HdiutilInfo>(programs::HDIUTIL, &["info", "-plist"]) {
                Ok(info) => image_paths(&info),
                Err(error) => {
                    failures.push(error.into_failure());
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };

        let devices = whole_disks
            .iter()
            .map(|disk| {
                let image = images
                    .get(disk.device_identifier.as_str())
                    .map(|path| DiskImage {
                        path: path.clone(),
                        stored_on: (self.mount_source)(Path::new(path)),
                    });
                build_device(disk, &infos, &containers, image)
            })
            .collect();

        Ok(DeviceListing {
            devices,
            command,
            failures,
        })
    }
}

enum PlistError {
    Command(CommandFailure),
    Parse { command: String, message: String },
}

impl PlistError {
    fn into_failure(self) -> CommandFailure {
        match self {
            Self::Command(failure) => failure,
            Self::Parse { command, message } => CommandFailure {
                command,
                stderr: message,
            },
        }
    }

    fn into_disk_info_error(self) -> DiskInfoError {
        match self {
            Self::Command(failure) => DiskInfoError::CommandFailed(failure),
            Self::Parse { command, message } => DiskInfoError::Parse { command, message },
        }
    }
}

/// ディスクイメージの全体のディスク（`disk4`）→ イメージファイルのパス
fn image_paths(info: &HdiutilInfo) -> HashMap<String, String> {
    info.images
        .iter()
        .flat_map(|image| {
            image.system_entities.iter().filter_map(move |entity| {
                let id = entity.dev_entry.strip_prefix("/dev/")?;
                is_whole_disk_id(id).then(|| (id.to_owned(), image.image_path.clone()))
            })
        })
        .collect()
}

/// スナップショットのBSD名（`disk3s1s1`。`s`で区切った数が3つ）か
fn is_snapshot_id(id: &str) -> bool {
    id.strip_prefix("disk")
        .is_some_and(|rest| rest.split('s').filter(|n| !n.is_empty()).count() >= 3)
}

fn is_whole_disk_id(id: &str) -> bool {
    id.strip_prefix("disk")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn build_device(
    disk: &ListDisk,
    infos: &HashMap<&str, Info>,
    containers: &HashMap<&str, &ListDisk>,
    image: Option<DiskImage>,
) -> Device {
    let default_info = Info::default();
    let info = infos
        .get(disk.device_identifier.as_str())
        .unwrap_or(&default_info);
    let kind = if image.is_some() || info.bus_protocol.as_deref() == Some(BUS_DISK_IMAGE) {
        DeviceKind::Image
    } else if info.internal == Some(true) {
        DeviceKind::Internal
    } else {
        DeviceKind::External
    };

    let mut internal_volumes = HashSet::new();
    let partitions: Vec<Partition> = disk
        .partitions
        .iter()
        .map(|part| {
            let part_info = infos
                .get(part.device_identifier.as_str())
                .unwrap_or(&default_info);
            let type_name = part.content.clone().unwrap_or_default();
            let kind = partition_kind(&type_name);
            let apfs = containers
                .get(part.device_identifier.as_str())
                .map(|container| {
                    internal_volumes.extend(
                        container
                            .apfs_volumes
                            .iter()
                            .filter(|v| v.os_internal || container.os_internal)
                            .map(|v| v.device_identifier.clone()),
                    );
                    build_container(container)
                });
            Partition {
                id: part.device_identifier.clone(),
                kind,
                type_name,
                filesystem: filesystem_label(part_info, kind),
                name: non_empty(part.volume_name.as_deref()),
                offset: part_info.partition_map_partition_offset,
                size: part.size,
                part_uuid: part.disk_uuid.as_deref().map(str::to_ascii_uppercase),
                mount_point: non_empty(part.mount_point.as_deref()),
                apfs,
            }
        })
        .collect();

    let labels = match &image {
        Some(image) => Path::new(&image.path)
            .file_name()
            .map(|name| vec![name.to_string_lossy().into_owned()])
            .unwrap_or_default(),
        None if disk.partitions.is_empty() => {
            non_empty(disk.volume_name.as_deref()).into_iter().collect()
        }
        None => choose_labels(&partitions, &internal_volumes),
    };
    let extra_label_count = if image.is_some() {
        0
    } else {
        labels.len().saturating_sub(1)
    };

    Device {
        id: disk.device_identifier.clone(),
        kind,
        labels,
        extra_label_count,
        vendor: if kind == DeviceKind::Image {
            None
        } else {
            non_empty(info.media_name.as_deref())
        },
        image,
        connection: non_empty(info.bus_protocol.as_deref()),
        scheme: partition_scheme(disk.content.as_deref()),
        size: disk.size,
        partitions,
    }
}

fn build_container(container: &ListDisk) -> ApfsContainer {
    ApfsContainer {
        id: container.device_identifier.clone(),
        volumes: container
            .apfs_volumes
            .iter()
            // スナップショット（`disk3s1s1`）は、ボリューム本体のマウント先として扱う
            .filter(|v| !is_snapshot_id(&v.device_identifier))
            .map(build_volume)
            .collect(),
    }
}

fn build_volume(volume: &ListVolume) -> ApfsVolume {
    // システムのボリュームは、ボリューム自体ではなくスナップショットが`/`にマウントされている
    let mount_point = non_empty(volume.mount_point.as_deref()).or_else(|| {
        volume
            .mounted_snapshots
            .iter()
            .find_map(|snapshot| non_empty(snapshot.snapshot_mount_point.as_deref()))
    });
    ApfsVolume {
        id: volume.device_identifier.clone(),
        name: volume.volume_name.clone().unwrap_or_default(),
        mount_point,
        used: volume.capacity_in_use,
    }
}

fn partition_kind(content: &str) -> PartitionKind {
    if content == "EFI" || content.eq_ignore_ascii_case(ESP_TYPE_GUID) {
        PartitionKind::Esp
    } else if content.starts_with("Apple_APFS") {
        // Apple_APFS・Apple_APFS_ISC・Apple_APFS_Recovery
        PartitionKind::Apfs
    } else if matches!(content, "Apple_HFS" | "Apple_HFSX") {
        PartitionKind::Hfs
    } else {
        PartitionKind::Other
    }
}

fn partition_scheme(content: Option<&str>) -> PartitionScheme {
    match content {
        Some("GUID_partition_scheme") => PartitionScheme::Gpt,
        Some("FDisk_partition_scheme") => PartitionScheme::Mbr,
        Some("Apple_partition_scheme") => PartitionScheme::Apm,
        _ => PartitionScheme::None,
    }
}

/// ファイルシステムの短い表示名（モックの表記：FAT32・APFS・HFS+・ExFAT・NTFS）
fn filesystem_label(info: &Info, kind: PartitionKind) -> Option<String> {
    let name = non_empty(info.filesystem_name.as_deref());
    match info.filesystem_type.as_deref() {
        Some("msdos") => Some(
            name.as_deref()
                .and_then(|n| n.strip_prefix("MS-DOS "))
                .unwrap_or("FAT")
                .to_owned(),
        ),
        Some("exfat") => Some("ExFAT".to_owned()),
        Some("hfs") => Some("HFS+".to_owned()),
        Some("apfs") => Some("APFS".to_owned()),
        Some("ntfs") => Some("NTFS".to_owned()),
        Some(other) => name.or_else(|| Some(other.to_owned())),
        // APFSの物理ストアは、パーティション自体にはファイルシステムがない
        None if kind == PartitionKind::Apfs => Some("APFS".to_owned()),
        None => None,
    }
}

/// ラベル名の候補を選ぶ（スペック2章）。ESPを除くパーティションごとに1つ。
///
/// - APFSは、コンテナのボリュームのうち`/`にマウントされるもの、なければ最初のマウント中のもの、
///   なければ最初のもの。OSが内部で使うボリューム（`internal_volumes`）は候補にしない
/// - APFS・HFS+以外で、マウントされていないもの（Microsoft Reservedなど）は候補にしない
fn choose_labels(partitions: &[Partition], internal_volumes: &HashSet<String>) -> Vec<String> {
    partitions
        .iter()
        .filter(|part| part.kind != PartitionKind::Esp)
        .filter_map(|part| {
            if let Some(container) = &part.apfs {
                let volumes: Vec<&ApfsVolume> = container
                    .volumes
                    .iter()
                    .filter(|v| !internal_volumes.contains(&v.id) && !v.name.is_empty())
                    .collect();
                let chosen = volumes
                    .iter()
                    .find(|v| v.mount_point.as_deref() == Some("/"))
                    .or_else(|| volumes.iter().find(|v| v.mount_point.is_some()))
                    .or_else(|| volumes.first())?;
                return Some(chosen.name.clone());
            }
            if part.kind == PartitionKind::Other && part.mount_point.is_none() {
                return None;
            }
            part.name.clone()
        })
        .collect()
}

/// `statfs`で、パスを保存しているボリュームのデバイス（`/dev/disk1s1`）を調べ、BSD名にする
pub fn statfs_mount_source(path: &Path) -> Option<String> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: c_pathはNUL終端の文字列で、statは書き込み先として十分な大きさを持つ
    let result = unsafe { libc::statfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    // SAFETY: statfsが成功したので初期化済み
    let stat = unsafe { stat.assume_init() };
    // SAFETY: f_mntfromnameはNUL終端の固定長の配列
    let from = unsafe { CStr::from_ptr(stat.f_mntfromname.as_ptr()) };
    from.to_str().ok()?.strip_prefix("/dev/").map(str::to_owned)
}

#[cfg(test)]
mod tests;
