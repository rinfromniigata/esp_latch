//! ESP Latchの補助プロセス（root）。許可リストの操作だけを、引数とパーティションを検証して実行する（imple 5章）。
//!
//! 起動の流れ：引数 → 一時ディレクトリの検査 → トークンを読んで消す → ソケットを作る → 本体のPIDの監視 → 接続の受け付け。
//! 接続は、一時ディレクトリの所有者と同じUIDの相手だけを受け付ける。本体が終了するか、`shutdown`を受けたら終了する。

pub mod args;
pub mod ops;
pub mod parent;
pub mod runner;
pub mod server;
pub mod setup;
pub mod trash;

use std::ffi::OsString;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use esp_latch_protocol::{SOCKET_FILE, TOKEN_FILE};

use ops::{Executor, StatfsVolumeFs};
use runner::SystemRunner;
use server::{Next, Server};

/// フォールバックのマウントポイントを作る場所
const VOLUMES_DIR: &str = "/Volumes";

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let args = args::parse(args)?;
    let owner = setup::check_dir(&args.dir).map_err(|e| format!("directory: {e}"))?;
    let token =
        setup::take_token(&args.dir.join(TOKEN_FILE), owner).map_err(|e| format!("token: {e}"))?;
    if !parent::is_alive(args.parent_pid) {
        return Err("parent process is not running".into());
    }
    let socket = args.dir.join(SOCKET_FILE);
    let listener = setup::bind_socket(&socket, owner).map_err(|e| format!("socket: {e}"))?;

    // 本体が終了したら、補助プロセスも終了する
    {
        let socket = socket.clone();
        let pid = args.parent_pid;
        std::thread::spawn(move || {
            let _ = parent::wait_for_exit(pid);
            exit(&socket);
        });
    }

    let server = Arc::new(Server::new(
        token,
        Executor::new(
            Arc::new(SystemRunner),
            Arc::new(StatfsVolumeFs),
            PathBuf::from(VOLUMES_DIR),
        ),
    ));
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        // 一時ディレクトリの所有者と違う相手は、何も読まずに切る
        let Ok(peer) = setup::peer_ids(&stream) else {
            continue;
        };
        if !setup::peer_allowed(peer, owner) {
            continue;
        }
        let server = Arc::clone(&server);
        let socket = socket.clone();
        std::thread::spawn(move || {
            let Ok(reader) = stream.try_clone() else {
                return;
            };
            if let Ok(Next::Shutdown) = server.serve(BufReader::new(reader), &stream, peer) {
                exit(&socket);
            }
        });
    }
    Ok(())
}

/// ソケットを消して終了する
fn exit(socket: &Path) -> ! {
    let _ = std::fs::remove_file(socket);
    std::process::exit(0);
}
