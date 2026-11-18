//! `HelperGateway`：補助プロセスの起動（初回の認証）、接続、要求の送受信、終了（imple 5章「起動と認証」）。
//!
//! 1. 最初の要求で`privileged-status: waiting_auth`を出す
//! 2. 一時ディレクトリ（0700）を作り、トークン（ランダム32バイト）をファイル（0600）に書く
//! 3. 補助プロセスを起動する（`HelperLauncher`）。キャンセルは`AuthCancelled`、それ以外の失敗は`Unavailable`
//! 4. ソケットが現れるのを待って接続し、`privileged-status: running`を出す。以後はアプリを閉じるまで使い回す
//! 5. アプリの終了時に`shutdown`を送る（補助プロセスは本体のPIDの終了でも自分で終了する）

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use esp_latch_protocol::{MAX_LINE, Op, Request, Response, SOCKET_FILE, TOKEN_FILE, encode_line};

use super::{
    HelperLauncher, Launch, LaunchError, PrivilegedError, PrivilegedErrorKind, PrivilegedOps,
    PrivilegedReply, PrivilegedStatus,
};
use crate::events::{AppEvent, EventSink};
use crate::oplog::LogResult;

/// ソケットが現れるまで待つ時間（認証のあと、補助プロセスはすぐにソケットを作る）
const SOCKET_TIMEOUT: Duration = Duration::from_secs(10);
/// 1つの要求の応答を待つ時間（取り出し・マウントは数十秒かかることがある）
const REPLY_TIMEOUT: Duration = Duration::from_secs(180);

pub struct HelperGateway {
    launcher: Box<dyn HelperLauncher>,
    events: Arc<dyn EventSink>,
    /// 一時ディレクトリを作る場所（`$TMPDIR`）
    temp_root: PathBuf,
    socket_timeout: Duration,
    /// 接続中の補助プロセス。要求は同時に1つだけ送る（ロックを兼ねる）
    session: Mutex<Option<Session>>,
}

struct Session {
    dir: PathBuf,
    token: String,
    next_id: u64,
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// 要求の送受信の失敗
enum SendError {
    /// 送る前に接続が切れていた（補助プロセスが終了していた）。要求は届いていない
    NotDelivered(io::Error),
    /// 送ったあとに応答がない。要求が実行されたかは分からない
    NoReply(io::Error),
}

impl HelperGateway {
    pub fn new(launcher: Box<dyn HelperLauncher>, events: Arc<dyn EventSink>) -> Self {
        Self::with_options(launcher, events, std::env::temp_dir(), SOCKET_TIMEOUT)
    }

    pub fn with_options(
        launcher: Box<dyn HelperLauncher>,
        events: Arc<dyn EventSink>,
        temp_root: PathBuf,
        socket_timeout: Duration,
    ) -> Self {
        Self {
            launcher,
            events,
            temp_root,
            socket_timeout,
            session: Mutex::new(None),
        }
    }

    fn set_status(&self, status: PrivilegedStatus) {
        self.events.emit(AppEvent::PrivilegedStatus(status));
    }

    /// 補助プロセスを起動して接続する
    fn start(&self) -> Result<(Session, Launch), PrivilegedError> {
        let unavailable = |message: String, launch: Option<Launch>| PrivilegedError {
            kind: PrivilegedErrorKind::Unavailable { message },
            launch,
        };
        let token = random_token().map_err(|e| unavailable(e.to_string(), None))?;
        let dir =
            create_private_dir(&self.temp_root).map_err(|e| unavailable(e.to_string(), None))?;
        // 失敗したら一時ディレクトリを消す
        let cleanup = DirGuard(Some(dir.clone()));
        write_token(&dir.join(TOKEN_FILE), &token).map_err(|e| unavailable(e.to_string(), None))?;

        let command = match self.launcher.launch(&dir) {
            Ok(command) => command,
            Err(LaunchError::Cancelled { command }) => {
                return Err(PrivilegedError {
                    kind: PrivilegedErrorKind::AuthCancelled,
                    launch: Some(Launch {
                        command,
                        result: LogResult::Cancelled,
                    }),
                });
            }
            Err(LaunchError::Failed { command, message }) => {
                return Err(unavailable(
                    message,
                    Some(Launch {
                        command,
                        result: LogResult::Fail,
                    }),
                ));
            }
        };
        let launch = Launch {
            command,
            result: LogResult::Ok,
        };
        let stream = match wait_for_socket(&dir.join(SOCKET_FILE), self.socket_timeout) {
            Ok(stream) => stream,
            Err(error) => return Err(unavailable(error.to_string(), Some(launch))),
        };
        let configured = stream
            .set_read_timeout(Some(REPLY_TIMEOUT))
            .and_then(|()| stream.try_clone());
        let writer = match configured {
            Ok(writer) => writer,
            Err(error) => return Err(unavailable(error.to_string(), Some(launch))),
        };
        cleanup.keep();
        Ok((
            Session {
                dir,
                token,
                next_id: 1,
                reader: BufReader::new(stream),
                writer,
            },
            launch,
        ))
    }
}

impl PrivilegedOps for HelperGateway {
    fn execute(&self, op: Op) -> Result<PrivilegedReply, PrivilegedError> {
        let mut session = self.session.lock().expect("helper session");
        let mut launch = None;
        // 前回の補助プロセスが終了していて要求が届かなければ、1回だけ起動し直す
        for attempt in 0..2 {
            if session.is_none() {
                self.set_status(PrivilegedStatus::WaitingAuth);
                match self.start() {
                    Ok((started, command)) => {
                        *session = Some(started);
                        launch = Some(command);
                        self.set_status(PrivilegedStatus::Running);
                    }
                    Err(error) => {
                        self.set_status(PrivilegedStatus::None);
                        return Err(error);
                    }
                }
            }
            let current = session.as_mut().expect("session started");
            match current.send(&op) {
                Ok(response) => return Ok(PrivilegedReply { launch, response }),
                Err(SendError::NotDelivered(_)) if attempt == 0 && launch.is_none() => {
                    *session = None;
                    self.set_status(PrivilegedStatus::None);
                }
                Err(SendError::NotDelivered(error) | SendError::NoReply(error)) => {
                    *session = None;
                    self.set_status(PrivilegedStatus::None);
                    return Err(PrivilegedError {
                        kind: PrivilegedErrorKind::Unavailable {
                            message: error.to_string(),
                        },
                        launch,
                    });
                }
            }
        }
        unreachable!("the second attempt always returns")
    }

    fn shutdown(&self) {
        let Ok(mut session) = self.session.lock() else {
            return;
        };
        if let Some(mut current) = session.take() {
            let _ = current.send(&Op::Shutdown);
            self.set_status(PrivilegedStatus::None);
        }
    }
}

impl Session {
    fn send(&mut self, op: &Op) -> Result<Response, SendError> {
        let id = self.next_id;
        self.next_id += 1;
        let request = Request {
            token: self.token.clone(),
            id,
            op: op.clone(),
        };
        self.writer
            .write_all(&encode_line(&request))
            .and_then(|()| self.writer.flush())
            .map_err(SendError::NotDelivered)?;
        let mut line = Vec::new();
        let read = (&mut self.reader)
            .take(MAX_LINE as u64 + 1)
            .read_until(b'\n', &mut line)
            .map_err(SendError::NoReply)?;
        if read == 0 || line.last() != Some(&b'\n') {
            return Err(SendError::NoReply(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "helper closed the connection",
            )));
        }
        let response: Response = serde_json::from_slice(&line)
            .map_err(|e| SendError::NoReply(io::Error::new(io::ErrorKind::InvalidData, e)))?;
        if response.id != id {
            return Err(SendError::NoReply(io::Error::new(
                io::ErrorKind::InvalidData,
                "response id mismatch",
            )));
        }
        Ok(response)
    }
}

/// 失敗したときに一時ディレクトリを消す
struct DirGuard(Option<PathBuf>);

impl DirGuard {
    fn keep(mut self) {
        self.0 = None;
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        if let Some(dir) = self.0.take() {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

/// ランダムな32バイトを16進の文字列にする
fn random_token() -> io::Result<String> {
    let mut bytes = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// `root`の下に、本体だけが使える一時ディレクトリ（0700）を作る
fn create_private_dir(root: &Path) -> io::Result<PathBuf> {
    let suffix = &random_token()?[..8];
    let dir = root.join(format!("esp-latch-{}-{suffix}", std::process::id()));
    DirBuilder::new().mode(0o700).create(&dir)?;
    // umaskに左右されないよう、作ったあとにも設定する
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}

fn write_token(path: &Path, token: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(token.as_bytes())
}

/// ソケットが現れて接続できるまで待つ
fn wait_for_socket(path: &Path, timeout: Duration) -> io::Result<UnixStream> {
    let deadline = Instant::now() + timeout;
    loop {
        match UnixStream::connect(path) {
            Ok(stream) => return Ok(stream),
            Err(error) if Instant::now() >= deadline => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("helper socket did not appear: {error}"),
                ));
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

#[cfg(test)]
mod tests;
