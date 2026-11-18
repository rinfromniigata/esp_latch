//! 接続ごとの要求の処理：1行ずつ読み、トークンと引数を確かめてから実行し、1行で応答する。

use std::io::{self, BufRead, Read, Write};

use esp_latch_protocol::{HelperError, MAX_LINE, Op, Request, Response, encode_line, tokens_match};

use crate::ops::Executor;
use crate::setup::Owner;

/// 1行を処理したあとの動き
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// 次の行を待つ
    Continue,
    /// 接続を切る（読めない要求・トークンの不一致）
    Close,
    /// 補助プロセスを終了する
    Shutdown,
}

pub struct Server {
    token: String,
    executor: Executor,
}

impl Server {
    pub fn new(token: String, executor: Executor) -> Self {
        Self { token, executor }
    }

    /// 1行の要求を処理する
    pub fn handle_line(&self, line: &[u8], peer: Owner) -> (Response, Next) {
        let Ok(request) = serde_json::from_slice::<Request>(line) else {
            return (
                Response::failure(0, Vec::new(), HelperError::BadRequest),
                Next::Close,
            );
        };
        if !tokens_match(&self.token, &request.token) {
            return (
                Response::failure(request.id, Vec::new(), HelperError::InvalidToken),
                Next::Close,
            );
        }
        if let Err(error) = request.op.validate() {
            return (
                Response::failure(request.id, Vec::new(), error),
                Next::Continue,
            );
        }
        if request.op == Op::Shutdown {
            return (Response::success(request.id, Vec::new()), Next::Shutdown);
        }
        let outcome = self.executor.execute(&request.op, peer);
        let response = Response {
            id: request.id,
            ok: outcome.result.is_ok(),
            commands: outcome.commands,
            freed_bytes: outcome.freed_bytes,
            error: outcome.result.err(),
        };
        (response, Next::Continue)
    }

    /// 接続が閉じる・切る・終了の要求が来るまで処理する
    pub fn serve(
        &self,
        mut reader: impl BufRead,
        mut writer: impl Write,
        peer: Owner,
    ) -> io::Result<Next> {
        let mut line = Vec::new();
        loop {
            if !read_line(&mut reader, &mut line)? {
                return Ok(Next::Close);
            }
            let (response, next) = self.handle_line(&line, peer);
            writer.write_all(&encode_line(&response))?;
            writer.flush()?;
            if next != Next::Continue {
                return Ok(next);
            }
        }
    }
}

/// 改行までの1行を読む（上限つき）。接続が閉じたら`false`、上限を超えたらエラー
fn read_line(reader: &mut impl BufRead, line: &mut Vec<u8>) -> io::Result<bool> {
    line.clear();
    reader
        .by_ref()
        .take(MAX_LINE as u64 + 1)
        .read_until(b'\n', line)?;
    match line.last() {
        None => Ok(false),
        Some(b'\n') => {
            line.pop();
            Ok(true)
        }
        Some(_) if line.len() > MAX_LINE => {
            Err(io::Error::new(io::ErrorKind::InvalidData, "line too long"))
        }
        // 改行なしで閉じた
        Some(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::ops::{MountInfo, VolumeFs};
    use crate::runner::fake::FakeRunner;

    const PEER: Owner = Owner { uid: 501, gid: 20 };

    struct NoVolumes;

    impl VolumeFs for NoVolumes {
        fn mount_info(&self, _: &std::path::Path) -> io::Result<MountInfo> {
            Err(io::ErrorKind::NotFound.into())
        }
    }

    fn server() -> (Server, Arc<FakeRunner>) {
        let runner = Arc::new(FakeRunner::default());
        runner.respond("diskutil eject disk4", 0, b"", "");
        let executor = Executor::new(
            runner.clone(),
            Arc::new(NoVolumes),
            PathBuf::from("/nonexistent"),
        );
        (Server::new("ab".repeat(32), executor), runner)
    }

    fn line(token: &str, id: u64, op: &str) -> String {
        format!(r#"{{"token":"{token}","id":{id},"op":{op}}}"#)
    }

    fn responses(output: &[u8]) -> Vec<Response> {
        output
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_slice(l).unwrap())
            .collect()
    }

    #[test]
    fn executes_requests_with_the_right_token() {
        let (server, runner) = server();
        let token = "ab".repeat(32);
        let input = format!(
            "{}\n{}\n",
            line(&token, 1, r#"{"kind":"eject_disk","disk":"disk4"}"#),
            line(&token, 2, r#"{"kind":"eject_disk","disk":"disk4s1"}"#)
        );
        let mut output = Vec::new();
        let next = server.serve(Cursor::new(input), &mut output, PEER).unwrap();
        assert_eq!(next, Next::Close);
        let responses = responses(&output);
        assert_eq!(responses.len(), 2);
        assert!(responses[0].ok);
        assert_eq!(responses[0].id, 1);
        assert_eq!(responses[0].commands[0].command, "diskutil eject disk4");
        assert_eq!(responses[1].error, Some(HelperError::InvalidTarget));
        assert_eq!(runner.calls(), ["diskutil eject disk4"]);
    }

    #[test]
    fn wrong_token_closes_the_connection_without_executing() {
        let (server, runner) = server();
        let input = format!(
            "{}\n{}\n",
            line(
                &"cd".repeat(32),
                1,
                r#"{"kind":"eject_disk","disk":"disk4"}"#
            ),
            line(
                &"ab".repeat(32),
                2,
                r#"{"kind":"eject_disk","disk":"disk4"}"#
            )
        );
        let mut output = Vec::new();
        let next = server.serve(Cursor::new(input), &mut output, PEER).unwrap();
        assert_eq!(next, Next::Close);
        let responses = responses(&output);
        assert_eq!(responses.len(), 1, "second request is never read");
        assert_eq!(responses[0].error, Some(HelperError::InvalidToken));
        assert!(runner.calls().is_empty());

        for token in ["", "ab", &"AB".repeat(32)] {
            let (response, next) =
                server.handle_line(line(token, 1, r#"{"kind":"shutdown"}"#).as_bytes(), PEER);
            assert_eq!(response.error, Some(HelperError::InvalidToken), "{token}");
            assert_eq!(next, Next::Close);
        }
    }

    #[test]
    fn unreadable_requests_close_the_connection() {
        let (server, runner) = server();
        for input in [
            "not json".to_owned(),
            line(&"ab".repeat(32), 1, r#"{"kind":"run","command":"sh"}"#),
            line(&"ab".repeat(32), 1, r#"{"kind":"empty_trash","path":"/"}"#),
        ] {
            let (response, next) = server.handle_line(input.as_bytes(), PEER);
            assert_eq!(response.error, Some(HelperError::BadRequest), "{input}");
            assert_eq!(next, Next::Close);
        }
        assert!(runner.calls().is_empty());
    }

    #[test]
    fn shutdown_is_acknowledged() {
        let (server, _) = server();
        let input = format!("{}\n", line(&"ab".repeat(32), 9, r#"{"kind":"shutdown"}"#));
        let mut output = Vec::new();
        let next = server.serve(Cursor::new(input), &mut output, PEER).unwrap();
        assert_eq!(next, Next::Shutdown);
        let responses = responses(&output);
        assert!(responses[0].ok);
        assert_eq!(responses[0].id, 9);
    }

    #[test]
    fn overlong_lines_are_rejected() {
        let (server, _) = server();
        let input = "x".repeat(MAX_LINE + 10);
        let mut output = Vec::new();
        assert!(server.serve(Cursor::new(input), &mut output, PEER).is_err());
        assert!(output.is_empty());
    }
}
