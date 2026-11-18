//! ドメインモデル。UIの`src/lib/api/types.ts`と一致させる（imple 3章）。
//!
//! 表示の文言は持たない。名前・種別・容量・識別子だけを持ち、UIが辞書と`Intl`で表示を組み立てる。

use serde::Serialize;

/// デバイスの種類（サイドバーのアイコンと絞り込みに使う）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Internal,
    External,
    Image,
}

/// パーティション方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionScheme {
    Gpt,
    Mbr,
    /// Apple Partition Map
    Apm,
    /// パーティションマップがない（ディスク全体が1つのボリューム）
    None,
}

/// 物理ディスクかディスクイメージ。APFSコンテナの合成ディスクは含めない（`Partition.apfs`の入れ子として持つ）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Device {
    /// BSD名（`disk2`）
    pub id: String,
    pub kind: DeviceKind,
    /// 表示名の候補。先頭が大きく出る名前。ディスクイメージはイメージのファイル名だけ。
    /// 空なら、UIは`vendor`を大きく出す
    pub labels: Vec<String>,
    /// 先頭のほかにあるラベルの数（「ほかN個」）
    pub extra_label_count: usize,
    /// メディア名（`diskutil info`の`MediaName`）。ディスクイメージは`None`
    pub vendor: Option<String>,
    pub image: Option<DiskImage>,
    /// 接続の種別（`diskutil info`の`BusProtocol`。USB・PCI-Express・Disk Imageなど。原文のまま見せる）
    pub connection: Option<String>,
    pub scheme: PartitionScheme,
    /// バイト数
    pub size: u64,
    pub partitions: Vec<Partition>,
}

/// ディスクイメージの情報
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskImage {
    /// イメージファイルのパス
    pub path: String,
    /// イメージファイルを保存しているボリュームのBSD名（`disk1s1`）。分からなければ`None`
    pub stored_on: Option<String>,
}

/// パーティションの種別（帯グラフの色と、ESPの操作の対象かどうかに使う）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionKind {
    Esp,
    Apfs,
    Hfs,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Partition {
    /// BSD名（`disk2s1`）
    pub id: String,
    pub kind: PartitionKind,
    /// `diskutil`が示すパーティションの種別名（`EFI`・`Apple_APFS`・`Microsoft Basic Data`など）
    pub type_name: String,
    /// ファイルシステムの短い表示名（`FAT32`・`APFS`・`HFS+`・`ExFAT`・`NTFS`など）。分からなければ`None`
    pub filesystem: Option<String>,
    /// ボリューム名。ないものは`None`（APFSの区画はコンテナの側に名前を持つ）
    pub name: Option<String>,
    /// ディスクの先頭からのバイト数。分からなければ`None`
    pub offset: Option<u64>,
    pub size: u64,
    /// GPTのパーティションUUID（大文字）。MBRなどでは`None`
    pub part_uuid: Option<String>,
    pub mount_point: Option<String>,
    /// APFSの区画のとき、その上のコンテナ
    pub apfs: Option<ApfsContainer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApfsContainer {
    /// コンテナの合成ディスクのBSD名（`disk3`）
    pub id: String,
    pub volumes: Vec<ApfsVolume>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApfsVolume {
    /// BSD名（`disk3s1`）
    pub id: String,
    pub name: String,
    /// マウント先。システムのボリュームは、スナップショットのマウント先（`/`）も含める
    pub mount_point: Option<String>,
    /// 使用量（バイト数）。分からなければ`None`
    pub used: Option<u64>,
}

/// 一覧の取得1回分の結果
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceSnapshot {
    pub devices: Vec<Device>,
    /// OpenCoreの起動パスから取ったGPTのパーティションUUID（大文字）。なければ`None`
    pub boot_part_uuid: Option<String>,
    /// 取得した時刻（UNIX時刻のミリ秒）
    pub taken_at: u64,
}

/// ESPの中のブートローダーの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoaderName {
    Opencore,
    Clover,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BootLoader {
    pub name: LoaderName,
    /// 版（OpenCoreは`1.0.8`、Cloverは`r5176`）。読めなければ`None`
    pub version: Option<String>,
}

/// ESPの容量（バイト数）。読めない値は`None`
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EspUsage {
    pub total: Option<u64>,
    pub free: Option<u64>,
    /// ゴミ箱（`.Trashes`）が占めている量
    pub trash: Option<u64>,
}

/// マウント中のESPの詳細（S01a・S01k）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EspDetail {
    /// パーティションのBSD名（`disk2s1`）
    pub part: String,
    pub mount_point: String,
    /// ブートローダー。見つからなければ`None`
    pub loader: Option<BootLoader>,
    pub usage: EspUsage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_enums_in_snake_case() {
        let json = serde_json::to_string(&[DeviceKind::Internal, DeviceKind::Image]).unwrap();
        assert_eq!(json, r#"["internal","image"]"#);
        let json = serde_json::to_string(&[PartitionScheme::Gpt, PartitionScheme::None]).unwrap();
        assert_eq!(json, r#"["gpt","none"]"#);
        let json = serde_json::to_string(&PartitionKind::Esp).unwrap();
        assert_eq!(json, r#""esp""#);
    }
}
