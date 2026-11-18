use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

use esp_latch_protocol::{CommandRecord, HelperError, tokens_match};

use super::*;
use crate::events::RecordingSink;

/// 補助プロセスの代わりにスレッドで要求に答える起動役。
/// トークンのファイルを読んで消し、ソケットを作り、受け取った操作を記録する
#[derive(Clone, Default)]
struct ThreadLauncher {
    launches: Arc<AtomicUsize>,
    received: Arc<Mutex<Vec<Op>>>,
    /// 起動のたびに確かめた一時ディレクトリの属性（パーミッション、トークンのパーミッション）
    seen_modes: Arc<Mutex<Vec<(u32, u32)>>>,
    /// trueなら、最初の要求に答えずに接続を切る
    hang_up: Arc<Mutex<bool>>,
}

impl HelperLauncher for ThreadLauncher {
    fn launch(&self, dir: &Path) -> Result<String, LaunchError> {
        self.launches.fetch_add(1, Ordering::SeqCst);
        let dir_mode = fs::metadata(dir).unwrap().mode() & 0o777;
        let token_path = dir.join(TOKEN_FILE);
        let token_mode = fs::metadata(&token_path).unwrap().mode() & 0o777;
        self.seen_modes.lock().unwrap().push((dir_mode, token_mode));
        let token = fs::read_to_string(&token_path).unwrap();
        fs::remove_file(&token_path).unwrap();
        let listener = UnixListener::bind(dir.join(SOCKET_FILE)).unwrap();
        let received = Arc::clone(&self.received);
        let hang_up = *self.hang_up.lock().unwrap();
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut writer = stream.try_clone().unwrap();
            for line in BufReader::new(stream).lines() {
                let Ok(line) = line else { return };
                let request: Request = serde_json::from_str(&line).unwrap();
                if hang_up {
                    return;
                }
                assert!(tokens_match(&token, &request.token));
                received.lock().unwrap().push(request.op.clone());
                let response = match &request.op {
                    Op::MountEsp { part } => Response::success(
                        request.id,
                        vec![CommandRecord {
                            command: format!("diskutil mount {part}"),
                            status: Some(0),
                            stderr: String::new(),
                        }],
                    ),
                    Op::Shutdown => {
                        writer
                            .write_all(&encode_line(&Response::success(request.id, vec![])))
                            .unwrap();
                        return;
                    }
                    _ => Response::failure(request.id, vec![], HelperError::NotEsp),
                };
                writer.write_all(&encode_line(&response)).unwrap();
            }
        });
        Ok("osascript -e 'do shell script …'".into())
    }
}

/// 決めた結果だけを返す起動役（ソケットは作らない）
struct FailingLauncher(LaunchError);

impl HelperLauncher for FailingLauncher {
    fn launch(&self, _: &Path) -> Result<String, LaunchError> {
        Err(self.0.clone())
    }
}

/// 起動は成功するが、ソケットを作らない起動役
struct SilentLauncher;

impl HelperLauncher for SilentLauncher {
    fn launch(&self, _: &Path) -> Result<String, LaunchError> {
        Ok("osascript".into())
    }
}

fn gateway(
    launcher: impl HelperLauncher + 'static,
) -> (HelperGateway, Arc<RecordingSink>, tempfile::TempDir) {
    let sink = Arc::new(RecordingSink::default());
    let root = tempfile::tempdir().unwrap();
    let gateway = HelperGateway::with_options(
        Box::new(launcher),
        sink.clone(),
        root.path().to_path_buf(),
        Duration::from_millis(300),
    );
    (gateway, sink, root)
}

fn statuses(sink: &RecordingSink) -> Vec<PrivilegedStatus> {
    sink.take()
        .into_iter()
        .filter_map(|event| match event {
            AppEvent::PrivilegedStatus(status) => Some(status),
            _ => None,
        })
        .collect()
}

fn mount(part: &str) -> Op {
    Op::MountEsp { part: part.into() }
}

fn entries(root: &tempfile::TempDir) -> usize {
    fs::read_dir(root.path()).unwrap().count()
}

#[test]
fn launches_once_and_reuses_the_connection() {
    let launcher = ThreadLauncher::default();
    let (gateway, sink, root) = gateway(launcher.clone());

    let first = gateway.execute(mount("disk4s1")).unwrap();
    assert_eq!(
        first.launch,
        Some(Launch {
            command: "osascript -e 'do shell script …'".into(),
            result: LogResult::Ok
        })
    );
    assert!(first.response.ok);
    assert_eq!(first.response.commands[0].command, "diskutil mount disk4s1");
    assert_eq!(
        statuses(&sink),
        [PrivilegedStatus::WaitingAuth, PrivilegedStatus::Running]
    );
    assert_eq!(*launcher.seen_modes.lock().unwrap(), [(0o700, 0o600)]);

    let second = gateway.execute(mount("disk6s1")).unwrap();
    assert_eq!(second.launch, None, "no second authentication");
    assert!(statuses(&sink).is_empty());
    let third = gateway
        .execute(Op::UnmountEsp {
            part: "disk4s1".into(),
        })
        .unwrap();
    assert_eq!(third.response.error, Some(HelperError::NotEsp));
    assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
    assert_eq!(entries(&root), 1, "temp dir stays while running");

    gateway.shutdown();
    assert_eq!(statuses(&sink), [PrivilegedStatus::None]);
    assert_eq!(
        launcher.received.lock().unwrap().last(),
        Some(&Op::Shutdown)
    );
    assert_eq!(entries(&root), 0, "temp dir is removed on shutdown");
    // 起動していなければ何もしない
    gateway.shutdown();
    assert!(statuses(&sink).is_empty());
}

#[test]
fn cancelled_authentication_is_reported_and_retryable() {
    let (gateway, sink, root) = gateway(FailingLauncher(LaunchError::Cancelled {
        command: "osascript …".into(),
    }));
    let error = gateway.execute(mount("disk4s1")).unwrap_err();
    assert_eq!(error.kind, PrivilegedErrorKind::AuthCancelled);
    assert_eq!(
        error.launch,
        Some(Launch {
            command: "osascript …".into(),
            result: LogResult::Cancelled
        })
    );
    assert_eq!(
        statuses(&sink),
        [PrivilegedStatus::WaitingAuth, PrivilegedStatus::None]
    );
    assert_eq!(entries(&root), 0, "temp dir is removed");
    // 再試行すると、もう一度認証を求める
    assert!(gateway.execute(mount("disk4s1")).is_err());
    assert_eq!(
        statuses(&sink),
        [PrivilegedStatus::WaitingAuth, PrivilegedStatus::None]
    );
}

#[test]
fn launch_failure_is_unavailable() {
    let (gateway, sink, root) = gateway(FailingLauncher(LaunchError::Failed {
        command: "osascript …".into(),
        message: "boom".into(),
    }));
    let error = gateway.execute(mount("disk4s1")).unwrap_err();
    assert!(matches!(
        error.kind,
        PrivilegedErrorKind::Unavailable { .. }
    ));
    assert_eq!(error.launch.unwrap().result, LogResult::Fail);
    assert_eq!(
        statuses(&sink),
        [PrivilegedStatus::WaitingAuth, PrivilegedStatus::None]
    );
    assert_eq!(entries(&root), 0);
}

#[test]
fn missing_socket_times_out() {
    let (gateway, _sink, root) = gateway(SilentLauncher);
    let error = gateway.execute(mount("disk4s1")).unwrap_err();
    assert!(matches!(
        error.kind,
        PrivilegedErrorKind::Unavailable { .. }
    ));
    assert_eq!(error.launch.unwrap().result, LogResult::Ok);
    assert_eq!(entries(&root), 0);
}

#[test]
fn lost_helper_is_reported_then_relaunched() {
    let launcher = ThreadLauncher::default();
    *launcher.hang_up.lock().unwrap() = true;
    let (gateway, sink, _root) = gateway(launcher.clone());
    let error = gateway.execute(mount("disk4s1")).unwrap_err();
    assert!(matches!(
        error.kind,
        PrivilegedErrorKind::Unavailable { .. }
    ));
    assert_eq!(
        statuses(&sink),
        [
            PrivilegedStatus::WaitingAuth,
            PrivilegedStatus::Running,
            PrivilegedStatus::None
        ]
    );
    *launcher.hang_up.lock().unwrap() = false;
    let reply = gateway.execute(mount("disk4s1")).unwrap();
    assert!(reply.launch.is_some());
    assert_eq!(launcher.launches.load(Ordering::SeqCst), 2);
}

#[test]
fn concurrent_requests_share_one_launch() {
    let launcher = ThreadLauncher::default();
    let (gateway, _sink, _root) = gateway(launcher.clone());
    let gateway = Arc::new(gateway);
    let (done, results) = mpsc::channel();
    for part in ["disk4s1", "disk6s1", "disk8s1"] {
        let gateway = Arc::clone(&gateway);
        let done = done.clone();
        std::thread::spawn(move || done.send(gateway.execute(mount(part))).unwrap());
    }
    for _ in 0..3 {
        assert!(results.recv().unwrap().unwrap().response.ok);
    }
    assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
}

#[test]
fn token_is_random_hex() {
    let a = random_token().unwrap();
    let b = random_token().unwrap();
    assert_eq!(a.len(), esp_latch_protocol::TOKEN_LEN);
    assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, b);
}
