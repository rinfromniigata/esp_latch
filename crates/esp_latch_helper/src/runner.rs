//! 外部コマンドの実行。テストでは偽実装を渡す。rootで動くため、環境変数を空にして絶対パスで実行する。

use std::process::Command;

use esp_latch_protocol::CommandRecord;
use esp_latch_protocol::command::display_command;

pub mod programs {
    pub const DISKUTIL: &str = "/usr/sbin/diskutil";
    pub const MOUNT: &str = "/sbin/mount";
}

/// 実行の結果。`record`は本体の操作ログに渡す
#[derive(Debug, Clone)]
pub struct Output {
    pub record: CommandRecord,
    pub stdout: Vec<u8>,
}

pub trait Runner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Output;
}

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Output {
        let command = display_command(program, args);
        match Command::new(program)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .output()
        {
            Ok(output) => Output {
                record: CommandRecord {
                    command,
                    status: output.status.code(),
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                },
                stdout: output.stdout,
            },
            Err(error) => Output {
                record: CommandRecord {
                    command,
                    status: None,
                    stderr: error.to_string(),
                },
                stdout: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
pub mod fake {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    /// 終了コード・標準出力・標準エラー出力
    type Reply = (i32, Vec<u8>, String);

    /// コマンドの文字列ごとに決めておいた結果を返す。決めていないコマンドは失敗（status 1）
    #[derive(Default)]
    pub struct FakeRunner {
        responses: Mutex<HashMap<String, Reply>>,
        calls: Mutex<Vec<String>>,
    }

    impl FakeRunner {
        pub fn respond(&self, command: &str, status: i32, stdout: &[u8], stderr: &str) {
            self.responses.lock().unwrap().insert(
                command.to_owned(),
                (status, stdout.to_vec(), stderr.to_owned()),
            );
        }

        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl Runner for FakeRunner {
        fn run(&self, program: &str, args: &[&str]) -> Output {
            let command = display_command(program, args);
            self.calls.lock().unwrap().push(command.clone());
            let (status, stdout, stderr) = self
                .responses
                .lock()
                .unwrap()
                .get(&command)
                .cloned()
                .unwrap_or((1, Vec::new(), format!("no fake for {command}")));
            Output {
                record: CommandRecord {
                    command,
                    status: Some(status),
                    stderr,
                },
                stdout,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_with_an_empty_environment() {
        let output = SystemRunner.run("/usr/bin/env", &[]);
        assert!(output.record.succeeded());
        let env = String::from_utf8(output.stdout).unwrap();
        assert_eq!(env.trim(), "PATH=/usr/bin:/bin:/usr/sbin:/sbin");
    }

    #[test]
    fn reports_failures() {
        let output = SystemRunner.run("/bin/sh", &["-c", "echo bad >&2; exit 2"]);
        assert_eq!(output.record.status, Some(2));
        assert_eq!(output.record.stderr, "bad");
        let output = SystemRunner.run("/nonexistent", &[]);
        assert_eq!(output.record.status, None);
        assert!(!output.record.succeeded());
    }
}
