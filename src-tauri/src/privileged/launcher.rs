//! 補助プロセスの起動。`osascript`の`do shell script … with administrator privileges`で、管理者の認証を経てrootで起動する。
//!
//! `do shell script`はコマンドの終了まで戻らないため、補助プロセスをバックグラウンドで起動して即座に戻らせる。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use esp_latch_protocol::command::shell_quote;

use crate::runner::{CommandRunner, display_command};

const OSASCRIPT: &str = "/usr/bin/osascript";
/// 同梱の補助プロセスのファイル名（`bundle.externalBin`の名前からターゲットトリプルを除いたもの）
const HELPER_NAME: &str = "esp-latch-helper";
/// osascriptのエラーの番号：利用者がキャンセルした
const USER_CANCELED: &str = "(-128)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    /// 認証のダイアログでキャンセルされた
    Cancelled { command: String },
    /// 起動できなかった
    Failed { command: String, message: String },
}

/// 補助プロセスの起動。テストでは、スレッドで補助プロセスの代わりを立てる実装を渡す
pub trait HelperLauncher: Send + Sync {
    /// `dir`（トークンを置いた一時ディレクトリ）を渡して補助プロセスを起動する。戻り値は、操作ログに出すコマンド
    fn launch(&self, dir: &Path) -> Result<String, LaunchError>;
}

/// 本体の実行ファイルと同じ場所にある補助プロセス（`.app/Contents/MacOS/`、開発時は`target/debug/`）
pub fn helper_path() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
    Ok(dir.join(HELPER_NAME))
}

pub struct OsascriptLauncher {
    runner: Arc<dyn CommandRunner>,
    helper: PathBuf,
    parent_pid: u32,
}

impl OsascriptLauncher {
    pub fn new(runner: Arc<dyn CommandRunner>, helper: PathBuf, parent_pid: u32) -> Self {
        Self {
            runner,
            helper,
            parent_pid,
        }
    }

    /// `do shell script`に渡すAppleScriptの文
    fn script(&self, dir: &Path) -> String {
        let shell = format!(
            "{} --dir {} --parent-pid {} >/dev/null 2>&1 &",
            shell_quote(&self.helper.to_string_lossy()),
            shell_quote(&dir.to_string_lossy()),
            self.parent_pid
        );
        format!(
            "do shell script \"{}\" with administrator privileges",
            applescript_escape(&shell)
        )
    }
}

impl HelperLauncher for OsascriptLauncher {
    fn launch(&self, dir: &Path) -> Result<String, LaunchError> {
        let script = self.script(dir);
        let args = ["-e", script.as_str()];
        let command = display_command(OSASCRIPT, &args);
        match self.runner.run(OSASCRIPT, &args) {
            Ok(output) if output.success() => Ok(command),
            Ok(output) => {
                let stderr = output.stderr_text();
                if stderr.contains(USER_CANCELED) {
                    Err(LaunchError::Cancelled { command })
                } else {
                    Err(LaunchError::Failed {
                        command,
                        message: stderr,
                    })
                }
            }
            Err(error) => Err(LaunchError::Failed {
                command,
                message: error.to_string(),
            }),
        }
    }
}

/// AppleScriptの文字列の中に入れるため、`\`と`"`をエスケープする
fn applescript_escape(text: &str) -> String {
    text.replace('\\', r"\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::FakeRunner;

    fn launcher(runner: FakeRunner, helper: &str) -> OsascriptLauncher {
        OsascriptLauncher::new(Arc::new(runner), PathBuf::from(helper), 4242)
    }

    const HELPER: &str = "/Applications/ESP Latch.app/Contents/MacOS/esp-latch-helper";
    const DIR: &str = "/var/folders/ab/xyz/T/esp-latch-4242-0011aabb";

    fn expected_command() -> String {
        let script = "do shell script \"'/Applications/ESP Latch.app/Contents/MacOS/esp-latch-helper' --dir /var/folders/ab/xyz/T/esp-latch-4242-0011aabb --parent-pid 4242 >/dev/null 2>&1 &\" with administrator privileges";
        display_command(OSASCRIPT, &["-e", script])
    }

    #[test]
    fn launches_in_the_background_with_administrator_privileges() {
        let runner = FakeRunner::new();
        runner.respond(&expected_command(), 0, b"", "");
        let launcher = launcher(runner, HELPER);
        assert_eq!(launcher.launch(Path::new(DIR)), Ok(expected_command()));
        assert!(expected_command().starts_with("osascript -e 'do shell script"));
    }

    #[test]
    fn cancel_is_reported_separately() {
        let runner = FakeRunner::new();
        runner.respond(
            &expected_command(),
            1,
            b"",
            "0:96: execution error: User canceled. (-128)",
        );
        let launcher = launcher(runner, HELPER);
        assert_eq!(
            launcher.launch(Path::new(DIR)),
            Err(LaunchError::Cancelled {
                command: expected_command()
            })
        );
    }

    #[test]
    fn other_failures_are_failures() {
        let runner = FakeRunner::new();
        runner.respond(
            &expected_command(),
            1,
            b"",
            "execution error: The administrator user name or password was incorrect. (-60007)",
        );
        let launcher = launcher(runner, HELPER);
        assert!(matches!(
            launcher.launch(Path::new(DIR)),
            Err(LaunchError::Failed { .. })
        ));
        // osascriptを起動できない
        let launcher = self::launcher(FakeRunner::new(), HELPER);
        assert!(matches!(
            launcher.launch(Path::new(DIR)),
            Err(LaunchError::Failed { .. })
        ));
    }

    #[test]
    fn quotes_paths_for_the_shell_and_applescript() {
        let launcher = launcher(FakeRunner::new(), r#"/Apps/a "b" \c's/esp-latch-helper"#);
        let script = launcher.script(Path::new("/tmp/d"));
        assert_eq!(
            script,
            r#"do shell script "'/Apps/a \"b\" \\c'\\''s/esp-latch-helper' --dir /tmp/d --parent-pid 4242 >/dev/null 2>&1 &" with administrator privileges"#
        );
    }
}
