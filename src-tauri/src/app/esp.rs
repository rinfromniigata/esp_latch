//! ESPのユースケース（imple 4章）：`inspect_esp`・`mount_esp`・`unmount_esp`・`open_esp`。
//!
//! 権限が要る操作は`PrivilegedOps`に依頼し、補助プロセスが実行したコマンド（初回は起動の`osascript`も）を操作ログに記録する。
//! 同じディスクへの操作は同時に1つだけ（`begin_operation`）。マウント・取り外しのあとは一覧を取得し直す。

use std::path::Path;

use esp_latch_protocol::{HelperError, Op, bsd};

use super::{AppCore, AppError};
use crate::disk_info::{DiskInfoError, PartitionInfo};
use crate::domain::EspDetail;
use crate::events::Operation;
use crate::oplog::LogResult;
use crate::privileged::{PrivilegedErrorKind, Response};
use crate::runner::{display_command, programs};

impl AppCore {
    /// マウント中のESPの詳細（ブートローダー、容量）
    pub fn inspect_esp(&self, part: &str) -> Result<EspDetail, AppError> {
        let info = self.esp_partition(part)?;
        let mount_point = info.mount_point.clone().ok_or(AppError::NotMounted)?;
        Ok(self.esp_detail(&info, &mount_point))
    }

    /// ESPをマウントする。`open_finder`なら、マウント先をFinderで開く（設定の`open_finder_after_mount`）
    pub fn mount_esp(&self, part: &str, open_finder: bool) -> Result<EspDetail, AppError> {
        check_partition(part)?;
        let _operation = self.begin_operation(part, Operation::MountEsp)?;
        self.run_privileged(Op::MountEsp {
            part: part.to_owned(),
        })?;
        let info = self.esp_partition(part)?;
        let Some(mount_point) = info.mount_point.clone() else {
            let _ = self.refresh();
            return Err(AppError::NotMounted);
        };
        if open_finder {
            // Finderで開けなくても、マウントは成功として扱う（操作ログには失敗を残す）
            let _ = self.open_in_finder(&mount_point);
        }
        let detail = self.esp_detail(&info, &mount_point);
        let _ = self.refresh();
        Ok(detail)
    }

    /// マウント中のESPを取り外す
    pub fn unmount_esp(&self, part: &str) -> Result<(), AppError> {
        check_partition(part)?;
        let _operation = self.begin_operation(part, Operation::UnmountEsp)?;
        let result = self
            .run_privileged(Op::UnmountEsp {
                part: part.to_owned(),
            })
            .map(drop);
        // 失敗しても状態が変わっていることがあるため、取得し直す
        let _ = self.refresh();
        result
    }

    /// マウント先を引き直して、Finderで開く（UIからパスを受け取らない）
    pub fn open_esp(&self, part: &str) -> Result<(), AppError> {
        check_partition(part)?;
        let _operation = self.begin_operation(part, Operation::OpenEsp)?;
        let info = self.esp_partition(part)?;
        let mount_point = info.mount_point.ok_or(AppError::NotMounted)?;
        self.open_in_finder(&mount_point)
    }

    /// パーティションの情報を引き直し、ESPであることを確かめる
    fn esp_partition(&self, part: &str) -> Result<PartitionInfo, AppError> {
        check_partition(part)?;
        let info = self
            .ports
            .disk_info
            .partition(part)
            .map_err(|error| match error {
                // 見つからない（取り外された）パーティション
                DiskInfoError::CommandFailed(_) => AppError::InvalidTarget,
                other => AppError::from(other),
            })?;
        if info.is_esp {
            Ok(info)
        } else {
            Err(AppError::NotEsp)
        }
    }

    fn esp_detail(&self, info: &PartitionInfo, mount_point: &str) -> EspDetail {
        let path = Path::new(mount_point);
        // 起動に使ったESPなら、OpenCoreがNVRAMに書いた版を使える（ファイルからは版を読めないため）
        let booted = info.part_uuid.is_some() && info.part_uuid == self.boot_part_uuid();
        let opencore_version = if booted {
            self.opencore_version()
        } else {
            None
        };
        EspDetail {
            part: info.id.clone(),
            mount_point: mount_point.to_owned(),
            loader: self
                .ports
                .inspector
                .loader(path, opencore_version.as_deref()),
            usage: self.ports.inspector.usage(path),
        }
    }

    fn open_in_finder(&self, mount_point: &str) -> Result<(), AppError> {
        let args = [mount_point];
        match self.ports.runner.run_checked(programs::OPEN, &args) {
            Ok(_) => {
                self.log
                    .append(display_command(programs::OPEN, &args), LogResult::Ok);
                Ok(())
            }
            Err(failure) => {
                self.log.append(failure.command.clone(), LogResult::Fail);
                Err(failure.into())
            }
        }
    }

    /// 補助プロセスに依頼し、実行されたコマンドを操作ログに記録する
    fn run_privileged(&self, op: Op) -> Result<Response, AppError> {
        let reply = match self.ports.privileged.execute(op) {
            Ok(reply) => reply,
            Err(error) => {
                if let Some(launch) = error.launch {
                    self.log.append(launch.command, launch.result);
                }
                return Err(match error.kind {
                    PrivilegedErrorKind::AuthCancelled => AppError::AuthCancelled,
                    PrivilegedErrorKind::Unavailable { .. } => AppError::HelperUnavailable,
                });
            }
        };
        if let Some(launch) = reply.launch {
            self.log.append(launch.command, launch.result);
        }
        let response = reply.response;
        for record in &response.commands {
            let result = if record.succeeded() {
                LogResult::Ok
            } else {
                LogResult::Fail
            };
            self.log.append(record.command.clone(), result);
        }
        match (&response.error, response.ok) {
            (None, true) => Ok(response),
            (Some(error), _) => Err(helper_error(error.clone())),
            (None, false) => Err(AppError::HelperUnavailable),
        }
    }
}

fn check_partition(part: &str) -> Result<(), AppError> {
    if bsd::is_partition(part) {
        Ok(())
    } else {
        Err(AppError::InvalidTarget)
    }
}

fn helper_error(error: HelperError) -> AppError {
    match error {
        // 本体と補助プロセスの食い違い（通常は起きない）
        HelperError::BadRequest | HelperError::InvalidToken => AppError::HelperUnavailable,
        HelperError::InvalidTarget => AppError::InvalidTarget,
        HelperError::NotEsp => AppError::NotEsp,
        HelperError::NotMounted => AppError::NotMounted,
        HelperError::CommandFailed { command, stderr } => {
            AppError::CommandFailed { command, stderr }
        }
        HelperError::Io { message } => AppError::Io { message },
    }
}
