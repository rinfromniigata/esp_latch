//! 許可リストの操作の実行（imple 5章「各操作」）。
//!
//! ESPを対象にする操作は、`diskutil info -plist`を引き直して、パーティションの種別がEFIであることを確かめてから実行する。
//! 実行したコマンドは`CommandRecord`で返し、本体が操作ログに記録する。

use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use esp_latch_protocol::command::display_command;
use esp_latch_protocol::{CommandRecord, HelperError, Op, is_esp_content};
use serde::Deserialize;

use crate::runner::{Runner, programs};
use crate::setup::Owner;
use crate::trash::{self, TRASHES};

/// フォールバックで作るマウントポイントの名前（`EFI`、使われていれば`EFI 1`、`EFI 2`…）
const MOUNT_POINT_NAME: &str = "EFI";
const MAX_MOUNT_POINT_SUFFIX: u32 = 99;

/// 操作の結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub commands: Vec<CommandRecord>,
    pub freed_bytes: Option<u64>,
    pub result: Result<(), HelperError>,
}

impl Outcome {
    fn new(commands: Vec<CommandRecord>, result: Result<(), HelperError>) -> Self {
        Self {
            commands,
            freed_bytes: None,
            result,
        }
    }
}

/// マウント中のボリュームの情報（`statfs`）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountInfo {
    /// マウント元（`/dev/disk4s1`）
    pub from: String,
    /// マウント先
    pub on: PathBuf,
    /// 一般ユーザーが使える空き容量（バイト数）
    pub free: u64,
}

/// ボリュームの情報の取得。テストでは偽実装を渡す
pub trait VolumeFs: Send + Sync {
    fn mount_info(&self, path: &Path) -> io::Result<MountInfo>;
}

pub struct StatfsVolumeFs;

impl VolumeFs for StatfsVolumeFs {
    fn mount_info(&self, path: &Path) -> io::Result<MountInfo> {
        let c_path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: NUL終端のパスと、十分な大きさの書き込み先を渡す
        if unsafe { libc::statfs(c_path.as_ptr(), stat.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: statfsが成功したので初期化済み
        let stat = unsafe { stat.assume_init() };
        // SAFETY: f_mntfromname・f_mntonnameはNUL終端の固定長の配列
        let (from, on) = unsafe {
            (
                CStr::from_ptr(stat.f_mntfromname.as_ptr()),
                CStr::from_ptr(stat.f_mntonname.as_ptr()),
            )
        };
        Ok(MountInfo {
            from: from.to_string_lossy().into_owned(),
            on: PathBuf::from(on.to_string_lossy().into_owned()),
            free: stat.f_bavail.saturating_mul(u64::from(stat.f_bsize)),
        })
    }
}

/// `diskutil info -plist <part>`のうち、検証に使う項目
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PartitionInfo {
    device_identifier: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    mount_point: Option<String>,
}

pub struct Executor {
    runner: Arc<dyn Runner>,
    volumes: Arc<dyn VolumeFs>,
    /// フォールバックのマウントポイントを作る場所（`/Volumes`）
    volumes_dir: PathBuf,
    /// フォールバックで作ったマウントポイント。取り外したあと、空なら消す
    created: Mutex<HashSet<PathBuf>>,
}

impl Executor {
    pub fn new(runner: Arc<dyn Runner>, volumes: Arc<dyn VolumeFs>, volumes_dir: PathBuf) -> Self {
        Self {
            runner,
            volumes,
            volumes_dir,
            created: Mutex::new(HashSet::new()),
        }
    }

    /// 操作を実行する。`peer`は接続相手（フォールバックのマウントで、ファイルの所有者にする）。
    /// `shutdown`はここでは何もしない（接続の処理側で終了する）
    pub fn execute(&self, op: &Op, peer: Owner) -> Outcome {
        if let Err(error) = op.validate() {
            return Outcome::new(Vec::new(), Err(error));
        }
        match op {
            Op::MountEsp { part } => self.mount_esp(part, peer),
            Op::UnmountEsp { part } => self.unmount_esp(part),
            Op::EjectDisk { disk } => self.eject_disk(disk),
            Op::EmptyTrash { part } => self.empty_trash(part),
            Op::Shutdown => Outcome::new(Vec::new(), Ok(())),
        }
    }

    /// パーティションの情報を引き直し、ESPであることを確かめる。マウント先を返す
    fn esp_mount_point(&self, part: &str) -> Result<Option<PathBuf>, HelperError> {
        let output = self
            .runner
            .run(programs::DISKUTIL, &["info", "-plist", part]);
        if !output.record.succeeded() {
            return Err(HelperError::InvalidTarget);
        }
        let info: PartitionInfo =
            plist::from_bytes(&output.stdout).map_err(|_| HelperError::InvalidTarget)?;
        if info.device_identifier != part {
            return Err(HelperError::InvalidTarget);
        }
        if !info.content.as_deref().is_some_and(is_esp_content) {
            return Err(HelperError::NotEsp);
        }
        Ok(info
            .mount_point
            .filter(|path| !path.is_empty())
            .map(PathBuf::from))
    }

    /// マウント先が、確かにそのパーティションのものか（`statfs`のマウント元で確かめる）
    fn check_mounted_from(&self, mount_point: &Path, part: &str) -> Result<MountInfo, HelperError> {
        let info = self
            .volumes
            .mount_info(mount_point)
            .map_err(|_| HelperError::NotMounted)?;
        if info.from != format!("/dev/{part}") || info.on != mount_point {
            return Err(HelperError::InvalidTarget);
        }
        Ok(info)
    }

    fn mount_esp(&self, part: &str, peer: Owner) -> Outcome {
        match self.esp_mount_point(part) {
            Ok(Some(_)) => return Outcome::new(Vec::new(), Ok(())),
            Ok(None) => {}
            Err(error) => return Outcome::new(Vec::new(), Err(error)),
        }
        let first = self.runner.run(programs::DISKUTIL, &["mount", part]).record;
        if first.succeeded() {
            return Outcome::new(vec![first], Ok(()));
        }
        // diskutilでマウントできない（macOS 15のベータの事例）→ マウントポイントを作ってmount -t msdosに切り替える。
        // 通知には、diskutilのエラーを出す
        let failure = HelperError::CommandFailed {
            command: first.command.clone(),
            stderr: first.stderr.clone(),
        };
        let mut commands = vec![first];
        let Some(mount_point) = self.create_mount_point() else {
            return Outcome::new(commands, Err(failure));
        };
        let owner = format!("-u={},-g={}", peer.uid, peer.gid);
        let device = format!("/dev/{part}");
        let Some(target) = mount_point.to_str() else {
            let _ = std::fs::remove_dir(&mount_point);
            return Outcome::new(commands, Err(failure));
        };
        let fallback = self
            .runner
            .run(
                programs::MOUNT,
                &["-t", "msdos", "-o", &owner, &device, target],
            )
            .record;
        let mounted = fallback.succeeded();
        commands.push(fallback);
        if mounted {
            self.created.lock().expect("created").insert(mount_point);
            Outcome::new(commands, Ok(()))
        } else {
            let _ = std::fs::remove_dir(&mount_point);
            Outcome::new(commands, Err(failure))
        }
    }

    /// 使われていない`/Volumes/EFI`（`EFI 1`、`EFI 2`…）を作る
    fn create_mount_point(&self) -> Option<PathBuf> {
        (0..=MAX_MOUNT_POINT_SUFFIX).find_map(|suffix| {
            let name = if suffix == 0 {
                MOUNT_POINT_NAME.to_owned()
            } else {
                format!("{MOUNT_POINT_NAME} {suffix}")
            };
            let path = self.volumes_dir.join(name);
            // すでにあるもの（ファイル・リンクを含む）は使わない。作成はアトミックに行う
            std::fs::DirBuilder::new()
                .mode(0o755)
                .create(&path)
                .ok()
                .map(|()| path)
        })
    }

    fn unmount_esp(&self, part: &str) -> Outcome {
        match self.esp_mount_point(part) {
            Ok(Some(_)) => {}
            Ok(None) => return Outcome::new(Vec::new(), Err(HelperError::NotMounted)),
            Err(error) => return Outcome::new(Vec::new(), Err(error)),
        }
        let record = self
            .runner
            .run(programs::DISKUTIL, &["unmount", part])
            .record;
        let result = command_result(&record);
        if result.is_ok() {
            self.remove_created_mount_points();
        }
        Outcome::new(vec![record], result)
    }

    fn eject_disk(&self, disk: &str) -> Outcome {
        let record = self.runner.run(programs::DISKUTIL, &["eject", disk]).record;
        let result = command_result(&record);
        if result.is_ok() {
            self.remove_created_mount_points();
        }
        Outcome::new(vec![record], result)
    }

    fn empty_trash(&self, part: &str) -> Outcome {
        let mount_point = match self.esp_mount_point(part) {
            Ok(Some(path)) => path,
            Ok(None) => return Outcome::new(Vec::new(), Err(HelperError::NotMounted)),
            Err(error) => return Outcome::new(Vec::new(), Err(error)),
        };
        let before = match self.check_mounted_from(&mount_point, part) {
            Ok(info) => info.free,
            Err(error) => return Outcome::new(Vec::new(), Err(error)),
        };
        // ログには、モックの表記（rm -rf <マウント先>/.Trashes）で記録する
        let trashes = mount_point.join(TRASHES);
        let command = display_command("rm", &["-rf", &trashes.to_string_lossy()]);
        match trash::empty_trashes(&mount_point) {
            Ok(_) => {
                let after = self
                    .volumes
                    .mount_info(&mount_point)
                    .map_or(before, |info| info.free);
                let record = CommandRecord {
                    command,
                    status: Some(0),
                    stderr: String::new(),
                };
                Outcome {
                    commands: vec![record],
                    freed_bytes: Some(after.saturating_sub(before)),
                    result: Ok(()),
                }
            }
            Err(error) => {
                let message = error.to_string();
                let record = CommandRecord {
                    command,
                    status: Some(1),
                    stderr: message.clone(),
                };
                Outcome::new(vec![record], Err(HelperError::Io { message }))
            }
        }
    }

    /// フォールバックで作ったマウントポイントのうち、もうマウントされていないものを消す（空のときだけ）
    fn remove_created_mount_points(&self) {
        let mut created = self.created.lock().expect("created");
        created.retain(|path| {
            let still_mounted = self
                .volumes
                .mount_info(path)
                .is_ok_and(|info| info.on == *path);
            if still_mounted {
                return true;
            }
            match std::fs::remove_dir(path) {
                Ok(()) => false,
                Err(error) => error.kind() != io::ErrorKind::NotFound,
            }
        });
    }
}

fn command_result(record: &CommandRecord) -> Result<(), HelperError> {
    if record.succeeded() {
        Ok(())
    } else {
        Err(HelperError::CommandFailed {
            command: record.command.clone(),
            stderr: record.stderr.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
