//! 特権操作（imple 5章）。アプリ層は`PrivilegedOps`だけに依存する。
//!
//! 実装は`HelperGateway`（osascriptで補助プロセスを起動し、ソケットで要求する）。テストでは`FakePrivilegedOps`を渡す。

mod gateway;
mod launcher;

#[cfg(test)]
pub mod fake;

pub use esp_latch_protocol::{CommandRecord, HelperError, Op, Response};
pub use gateway::HelperGateway;
pub use launcher::{HelperLauncher, LaunchError, OsascriptLauncher, helper_path};

use serde::Serialize;

use crate::oplog::LogResult;

/// 補助プロセスの状態（`privileged-status`イベント）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegedStatus {
    /// 起動していない
    None,
    /// 管理者の認証を待っている
    WaitingAuth,
    /// 管理者権限で実行中
    Running,
}

/// この呼び出しで補助プロセスを起動したときの、起動のコマンド（`osascript …`）と結果。操作ログに記録する
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub command: String,
    pub result: LogResult,
}

/// 補助プロセスの応答
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivilegedReply {
    pub launch: Option<Launch>,
    pub response: Response,
}

/// 補助プロセスに要求を届けられなかった
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivilegedError {
    pub kind: PrivilegedErrorKind,
    pub launch: Option<Launch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivilegedErrorKind {
    /// 管理者の認証がキャンセルされた
    AuthCancelled,
    /// 補助プロセスを起動・接続できない、または途中で応答がなくなった。`message`は調査用
    Unavailable { message: String },
}

/// 特権操作
pub trait PrivilegedOps: Send + Sync {
    /// 補助プロセスに操作を依頼する。初回は補助プロセスを起動する（管理者の認証）
    fn execute(&self, op: Op) -> Result<PrivilegedReply, PrivilegedError>;

    /// 補助プロセスを終了する（アプリの終了時）。起動していなければ何もしない
    fn shutdown(&self);
}
