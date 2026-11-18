//! `diskutil`・`hdiutil`のplistの、使う項目だけの型。キーは出力のまま（ロケールに左右されない）。

use serde::Deserialize;

/// `diskutil list -plist`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct List {
    pub all_disks_and_partitions: Vec<ListDisk>,
    pub whole_disks: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListDisk {
    pub device_identifier: String,
    pub content: Option<String>,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub partitions: Vec<ListPartition>,
    /// APFSコンテナの合成ディスクのとき、その物理ストア
    #[serde(rename = "APFSPhysicalStores", default)]
    pub apfs_physical_stores: Vec<PhysicalStore>,
    #[serde(rename = "APFSVolumes", default)]
    pub apfs_volumes: Vec<ListVolume>,
    #[serde(rename = "OSInternal", default)]
    pub os_internal: bool,
    /// パーティションマップのないディスクのボリューム名
    pub volume_name: Option<String>,
}

impl ListDisk {
    /// APFSコンテナの合成ディスクかどうか
    pub fn is_apfs_container(&self) -> bool {
        !self.apfs_physical_stores.is_empty()
            || self.content.as_deref() == Some("Apple_APFS_Container")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PhysicalStore {
    pub device_identifier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListPartition {
    pub device_identifier: String,
    pub content: Option<String>,
    #[serde(default)]
    pub size: u64,
    /// GPTのパーティションUUID
    #[serde(rename = "DiskUUID")]
    pub disk_uuid: Option<String>,
    pub volume_name: Option<String>,
    pub mount_point: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListVolume {
    pub device_identifier: String,
    pub volume_name: Option<String>,
    pub mount_point: Option<String>,
    pub capacity_in_use: Option<u64>,
    #[serde(rename = "OSInternal", default)]
    pub os_internal: bool,
    #[serde(default)]
    pub mounted_snapshots: Vec<MountedSnapshot>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MountedSnapshot {
    pub snapshot_mount_point: Option<String>,
}

/// `diskutil info -plist <disk>`
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Info {
    pub bus_protocol: Option<String>,
    pub internal: Option<bool>,
    pub media_name: Option<String>,
    pub filesystem_type: Option<String>,
    pub filesystem_name: Option<String>,
    pub partition_map_partition_offset: Option<u64>,
}

/// `hdiutil info -plist`
#[derive(Debug, Deserialize)]
pub struct HdiutilInfo {
    #[serde(default)]
    pub images: Vec<HdiutilImage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HdiutilImage {
    pub image_path: String,
    #[serde(default)]
    pub system_entities: Vec<SystemEntity>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SystemEntity {
    pub dev_entry: String,
}

/// 空文字列を`None`にする（`diskutil info`は、ない値を空文字列で返す）
pub fn non_empty(value: Option<&str>) -> Option<String> {
    value.filter(|v| !v.is_empty()).map(str::to_owned)
}
