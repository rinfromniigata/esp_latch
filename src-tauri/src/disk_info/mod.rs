//! ディスク情報の読み取り。`DiskInfo`の実装は`DiskutilAdapter`（`diskutil`・`hdiutil`のplistから組み立てる）。

mod diskutil;
mod raw;

pub use diskutil::{DiskutilAdapter, MountSource, statfs_mount_source};

use crate::domain::Device;
use crate::runner::CommandFailure;

/// デバイスの一覧の取得1回分
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceListing {
    pub devices: Vec<Device>,
    /// 一覧の取得そのもののコマンド（`diskutil list -plist`）。操作ログにはこの1行だけを残す
    pub command: String,
    /// 途中で失敗した補助的なコマンド（`diskutil info`・`hdiutil info`）。
    /// 失敗した項目は分かる範囲で埋め、コマンドは操作ログに`fail`で別の行として残す
    pub failures: Vec<CommandFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiskInfoError {
    /// 一覧の取得のコマンドが失敗した
    #[error("{} failed: {}", .0.command, .0.stderr)]
    CommandFailed(CommandFailure),
    /// 出力のplistを読めなかった
    #[error("{command}: unexpected output: {message}")]
    Parse { command: String, message: String },
}

/// ディスク情報の読み取り
pub trait DiskInfo: Send + Sync {
    fn list_devices(&self) -> Result<DeviceListing, DiskInfoError>;
}
