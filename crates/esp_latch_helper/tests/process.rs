//! 補助プロセスを実際に起動して確かめる（root権限は使わない。許可リストの検証・終了の動きだけ）。

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use esp_latch_protocol::{
    HelperError, Op, Request, Response, SOCKET_FILE, TOKEN_FILE, encode_line,
};

const HELPER: &str = env!("CARGO_BIN_EXE_esp-latch-helper");

fn private_dir(token: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(dir.path().join(TOKEN_FILE))
        .unwrap();
    file.write_all(token.as_bytes()).unwrap();
    dir
}

fn spawn_helper(dir: &Path, parent_pid: u32) -> Child {
    Command::new(HELPER)
        .arg("--dir")
        .arg(dir)
        .arg("--parent-pid")
        .arg(parent_pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn connect(dir: &Path) -> UnixStream {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(stream) = UnixStream::connect(dir.join(SOCKET_FILE)) {
            return stream;
        }
        assert!(Instant::now() < deadline, "socket did not appear");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn request(stream: &mut UnixStream, token: &str, id: u64, op: Op) -> Option<Response> {
    let request = Request {
        token: token.into(),
        id,
        op,
    };
    stream.write_all(&encode_line(&request)).ok()?;
    let mut line = String::new();
    BufReader::new(stream.try_clone().unwrap())
        .read_line(&mut line)
        .ok()?;
    serde_json::from_str(&line).ok()
}

fn wait_exit(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(Instant::now() < deadline, "helper did not exit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn serves_requests_and_shuts_down() {
    let token = "0f".repeat(32);
    let dir = private_dir(&token);
    let mut helper = spawn_helper(dir.path(), std::process::id());
    let mut stream = connect(dir.path());
    assert!(
        !dir.path().join(TOKEN_FILE).exists(),
        "token file is removed"
    );
    let socket = fs::symlink_metadata(dir.path().join(SOCKET_FILE)).unwrap();
    assert_eq!(socket.permissions().mode() & 0o777, 0o600);

    let response = request(
        &mut stream,
        &token,
        1,
        Op::EjectDisk {
            disk: "disk4s1".into(),
        },
    )
    .unwrap();
    assert_eq!(response.id, 1);
    assert_eq!(response.error, Some(HelperError::InvalidTarget));

    let response = request(&mut stream, &token, 2, Op::Shutdown).unwrap();
    assert!(response.ok);
    assert!(wait_exit(&mut helper).success());
    assert!(!dir.path().join(SOCKET_FILE).exists(), "socket is removed");
}

#[test]
fn wrong_token_is_refused() {
    let token = "1e".repeat(32);
    let dir = private_dir(&token);
    let mut helper = spawn_helper(dir.path(), std::process::id());
    let mut stream = connect(dir.path());
    let response = request(&mut stream, &"2d".repeat(32), 1, Op::Shutdown).unwrap();
    assert_eq!(response.error, Some(HelperError::InvalidToken));
    // 接続は切られ、補助プロセスは終了しない
    assert!(request(&mut stream, &token, 2, Op::Shutdown).is_none());
    assert!(helper.try_wait().unwrap().is_none());

    let mut stream = connect(dir.path());
    assert!(request(&mut stream, &token, 3, Op::Shutdown).unwrap().ok);
    wait_exit(&mut helper);
}

#[test]
fn exits_when_the_parent_exits() {
    let token = "3c".repeat(32);
    let dir = private_dir(&token);
    let mut parent = Command::new("/bin/sleep").arg("30").spawn().unwrap();
    let mut helper = spawn_helper(dir.path(), parent.id());
    connect(dir.path());
    assert!(helper.try_wait().unwrap().is_none());
    parent.kill().unwrap();
    parent.wait().unwrap();
    assert!(wait_exit(&mut helper).success());
    assert!(!dir.path().join(SOCKET_FILE).exists());
}

#[test]
fn refuses_to_start_with_an_open_directory() {
    let token = "4b".repeat(32);
    let dir = private_dir(&token);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let mut helper = spawn_helper(dir.path(), std::process::id());
    assert!(!wait_exit(&mut helper).success());
    assert!(!dir.path().join(SOCKET_FILE).exists());
}

#[test]
fn refuses_to_start_without_a_running_parent() {
    let token = "5a".repeat(32);
    let dir = private_dir(&token);
    let mut finished = Command::new("/usr/bin/true").spawn().unwrap();
    let pid = finished.id();
    finished.wait().unwrap();
    let mut helper = spawn_helper(dir.path(), pid);
    assert!(!wait_exit(&mut helper).success());
    assert!(!dir.path().join(SOCKET_FILE).exists());
}
