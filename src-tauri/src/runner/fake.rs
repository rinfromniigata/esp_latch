//! テスト用の`CommandRunner`。コマンドの文字列ごとに、決めておいた結果を返す。

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use super::{CommandOutput, CommandRunner, display_command};

#[derive(Debug, Default)]
pub struct FakeRunner {
    responses: Mutex<HashMap<String, CommandOutput>>,
    calls: Mutex<Vec<String>>,
}

impl FakeRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// フィクスチャのディレクトリから作る。`diskutil_list.plist`は`diskutil list -plist`、
    /// `diskutil_info_disk2.plist`は`diskutil info -plist disk2`、`hdiutil_info.plist`は`hdiutil info -plist`の出力
    pub fn from_fixture_dir(dir: &Path) -> Self {
        let runner = Self::new();
        for entry in std::fs::read_dir(dir).expect("fixture dir") {
            let path = entry.expect("fixture entry").path();
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let command = if stem == "diskutil_list" {
                "diskutil list -plist".to_owned()
            } else if stem == "hdiutil_info" {
                "hdiutil info -plist".to_owned()
            } else if let Some(disk) = stem.strip_prefix("diskutil_info_") {
                format!("diskutil info -plist {disk}")
            } else {
                continue;
            };
            runner.respond(&command, 0, &std::fs::read(&path).expect("fixture"), "");
        }
        runner
    }

    /// `command`（表示用の文字列）に対する結果を決める
    pub fn respond(&self, command: &str, status: i32, stdout: &[u8], stderr: &str) {
        self.responses.lock().unwrap().insert(
            command.to_owned(),
            CommandOutput {
                status: Some(status),
                stdout: stdout.to_vec(),
                stderr: stderr.as_bytes().to_vec(),
            },
        );
    }

    /// 呼ばれたコマンドの文字列（呼ばれた順）
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput> {
        let command = display_command(program, args);
        self.calls.lock().unwrap().push(command.clone());
        self.responses
            .lock()
            .unwrap()
            .get(&command)
            .cloned()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("no fake for {command}"))
            })
    }
}
