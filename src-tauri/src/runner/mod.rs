//! 外部コマンドの実行。アプリ層と各アダプタは`CommandRunner`だけに依存し、テストでは偽実装を渡す。

use std::io;
use std::process::Command;

#[cfg(test)]
pub mod fake;

/// コマンドの実行結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// 終了コード。シグナルで終わった場合は`None`
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.status == Some(0)
    }

    /// 標準エラー出力を文字列にする（前後の空白を除く）
    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_owned()
    }
}

/// 失敗したコマンド。操作ログとエラーの通知に、原文のまま見せる
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFailure {
    /// 実行したコマンドの表示用の文字列（`diskutil info -plist disk2`）
    pub command: String,
    /// 標準エラー出力。起動に失敗した場合はそのエラー
    pub stderr: String,
}

/// 外部コマンドの実行
pub trait CommandRunner: Send + Sync {
    /// `program`を`args`で実行し、終了を待つ。起動できなかった場合だけ`Err`を返す
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput>;

    /// 実行し、終了コードが0でなければ`CommandFailure`にする
    fn run_checked(&self, program: &str, args: &[&str]) -> Result<CommandOutput, CommandFailure> {
        let command = display_command(program, args);
        match self.run(program, args) {
            Ok(output) if output.success() => Ok(output),
            Ok(output) => Err(CommandFailure {
                command,
                stderr: output.stderr_text(),
            }),
            Err(error) => Err(CommandFailure {
                command,
                stderr: error.to_string(),
            }),
        }
    }
}

/// 実際にプロセスを起動する実装
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput> {
        let output = Command::new(program).args(args).output()?;
        Ok(CommandOutput {
            status: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

/// 使う外部コマンドの絶対パス（`PATH`に左右されないようにする）
pub mod programs {
    pub const DISKUTIL: &str = "/usr/sbin/diskutil";
    pub const HDIUTIL: &str = "/usr/bin/hdiutil";
    pub const NVRAM: &str = "/usr/sbin/nvram";
    pub const OPEN: &str = "/usr/bin/open";
}

/// 操作ログに出すコマンドの文字列（補助プロセスと同じ表記）
pub use esp_latch_protocol::command::display_command;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_runner_reports_status_and_stderr() {
        let output = SystemRunner
            .run("/bin/sh", &["-c", "echo out; echo err >&2; exit 3"])
            .unwrap();
        assert_eq!(output.status, Some(3));
        assert_eq!(output.stdout, b"out\n");
        assert_eq!(output.stderr_text(), "err");
    }

    #[test]
    fn run_checked_turns_failures_into_command_failure() {
        let failure = SystemRunner
            .run_checked("/bin/sh", &["-c", "echo bad >&2; exit 1"])
            .unwrap_err();
        assert_eq!(failure.command, "sh -c 'echo bad >&2; exit 1'");
        assert_eq!(failure.stderr, "bad");

        let failure = SystemRunner
            .run_checked("/nonexistent/program", &[])
            .unwrap_err();
        assert_eq!(failure.command, "program");
        assert!(!failure.stderr.is_empty());
    }
}
