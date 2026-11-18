//! アプリ層のエラー。UIは`kind`を辞書のキーに対応づけ、文言はUIの辞書で引く（imple 4章）。

use serde::Serialize;

use crate::disk_info::DiskInfoError;
use crate::runner::CommandFailure;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppError {
    /// 管理者権限の認証がキャンセルされた
    #[error("authentication cancelled")]
    AuthCancelled,
    /// コマンドが失敗した。`command`と`stderr`は原文のまま見せる
    #[error("{command} failed: {stderr}")]
    CommandFailed { command: String, stderr: String },
    /// 対象がESPではない
    #[error("not an EFI system partition")]
    NotEsp,
    /// 引数のBSD名の形式が違う、または対象が見つからない
    #[error("invalid target")]
    InvalidTarget,
    /// 同じディスクへの操作を実行中
    #[error("another operation is running on the same disk")]
    Busy,
    /// 補助プロセスを起動・接続できない
    #[error("privileged helper unavailable")]
    HelperUnavailable,
    /// マウントされていない
    #[error("not mounted")]
    NotMounted,
    /// 入出力のエラー（設定ファイルの書き込みなど）。`message`は調査用で、UIには出さない
    #[error("i/o error: {message}")]
    Io { message: String },
}

impl From<CommandFailure> for AppError {
    fn from(failure: CommandFailure) -> Self {
        Self::CommandFailed {
            command: failure.command,
            stderr: failure.stderr,
        }
    }
}

impl From<DiskInfoError> for AppError {
    fn from(error: DiskInfoError) -> Self {
        match error {
            DiskInfoError::CommandFailed(failure) => failure.into(),
            DiskInfoError::Parse { command, message } => Self::CommandFailed {
                command,
                stderr: message,
            },
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_kind_tag() {
        assert_eq!(
            serde_json::to_string(&AppError::Busy).unwrap(),
            r#"{"kind":"busy"}"#
        );
        assert_eq!(
            serde_json::to_string(&AppError::AuthCancelled).unwrap(),
            r#"{"kind":"auth_cancelled"}"#
        );
        let error = AppError::from(CommandFailure {
            command: "diskutil mount disk4s1".into(),
            stderr: "Volume on disk4s1 failed to mount".into(),
        });
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"kind":"command_failed","command":"diskutil mount disk4s1","stderr":"Volume on disk4s1 failed to mount"}"#
        );
    }
}
