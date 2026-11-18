//! 起動時の検査：一時ディレクトリ・トークン・ソケット・接続相手（imple 5章）。
//!
//! 一時ディレクトリは本体（一般ユーザー）が作ったもので、実体のディレクトリ（シンボリックリンクでない）、
//! 所有者がroot以外、パーミッションが0700であること。トークンのファイルは読んだら消す。
//! ソケットはディレクトリの所有者のものにして0600にし、接続ごとに相手のUIDを確かめる。

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use esp_latch_protocol::TOKEN_LEN;

/// 一時ディレクトリの所有者。接続を許す相手と、ソケットの所有者
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owner {
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetupError {
    /// シンボリックリンク
    Symlink,
    /// ディレクトリ・通常のファイルではない
    WrongType,
    /// 所有者がroot、または所有者がディレクトリと違う
    WrongOwner,
    /// パーミッションが決めた値ではない
    WrongMode(u32),
    /// トークンの形式が違う
    InvalidToken,
    Io(io::ErrorKind),
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Symlink => f.write_str("symbolic link is not allowed"),
            Self::WrongType => f.write_str("unexpected file type"),
            Self::WrongOwner => f.write_str("unexpected owner"),
            Self::WrongMode(mode) => write!(f, "unexpected mode {mode:o}"),
            Self::InvalidToken => f.write_str("invalid token"),
            Self::Io(kind) => write!(f, "i/o error: {kind}"),
        }
    }
}

impl From<io::Error> for SetupError {
    fn from(error: io::Error) -> Self {
        // O_NOFOLLOWでシンボリックリンクを開くとELOOPになる
        if error.raw_os_error() == Some(libc::ELOOP) {
            Self::Symlink
        } else {
            Self::Io(error.kind())
        }
    }
}

/// 一時ディレクトリを検査し、所有者を返す
pub fn check_dir(dir: &Path) -> Result<Owner, SetupError> {
    let meta = fs::symlink_metadata(dir)?;
    check_dir_metadata(
        meta.file_type().is_symlink(),
        meta.is_dir(),
        meta.uid(),
        meta.mode(),
    )?;
    Ok(Owner {
        uid: meta.uid(),
        gid: meta.gid(),
    })
}

/// 一時ディレクトリの属性の検査（所有者がrootの場合を含めて単体テストするため、値で受け取る）
pub fn check_dir_metadata(
    is_symlink: bool,
    is_dir: bool,
    uid: u32,
    mode: u32,
) -> Result<(), SetupError> {
    if is_symlink {
        return Err(SetupError::Symlink);
    }
    if !is_dir {
        return Err(SetupError::WrongType);
    }
    if uid == 0 {
        return Err(SetupError::WrongOwner);
    }
    if mode & 0o7777 != 0o700 {
        return Err(SetupError::WrongMode(mode & 0o7777));
    }
    Ok(())
}

/// トークンのファイルを読んで消す。シンボリックリンクはたどらず、所有者がディレクトリと同じで0600の通常のファイルに限る
pub fn take_token(path: &Path, owner: Owner) -> Result<String, SetupError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    // 検査に通らなくても、トークンは使えないものとして消す
    let removed = fs::remove_file(path);
    if !meta.is_file() {
        return Err(SetupError::WrongType);
    }
    if meta.uid() != owner.uid {
        return Err(SetupError::WrongOwner);
    }
    if meta.mode() & 0o7777 != 0o600 {
        return Err(SetupError::WrongMode(meta.mode() & 0o7777));
    }
    removed?;
    let mut token = String::new();
    file.by_ref()
        .take(TOKEN_LEN as u64 + 2)
        .read_to_string(&mut token)
        .map_err(|_| SetupError::InvalidToken)?;
    let token = token.trim_end_matches('\n');
    if token.len() != TOKEN_LEN || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(SetupError::InvalidToken);
    }
    Ok(token.to_owned())
}

/// ソケットを作り、所有者をディレクトリの所有者にして0600にする
pub fn bind_socket(path: &Path, owner: Owner) -> Result<UnixListener, SetupError> {
    let listener = UnixListener::bind(path)?;
    let secured = std::os::unix::fs::chown(path, Some(owner.uid), Some(owner.gid))
        .and_then(|()| fs::set_permissions(path, fs::Permissions::from_mode(0o600)));
    if let Err(error) = secured {
        let _ = fs::remove_file(path);
        return Err(error.into());
    }
    Ok(listener)
}

/// 接続相手のUID・GID（`getpeereid`）
pub fn peer_ids(stream: &UnixStream) -> io::Result<Owner> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: 有効なソケットのfdと、書き込み先の変数を渡す
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result == 0 {
        Ok(Owner { uid, gid })
    } else {
        Err(io::Error::last_os_error())
    }
}

/// 接続を許す相手か（一時ディレクトリの所有者と同じUID）
pub fn peer_allowed(peer: Owner, owner: Owner) -> bool {
    peer.uid == owner.uid
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, symlink};

    fn private_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        dir
    }

    fn write_token(path: &Path, contents: &str, mode: u32) {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true).mode(mode);
        std::io::Write::write_all(&mut options.open(path).unwrap(), contents.as_bytes()).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn accepts_private_directory_of_a_regular_user() {
        let dir = private_dir();
        let owner = check_dir(dir.path()).unwrap();
        assert_eq!(owner.uid, unsafe { libc::getuid() });
    }

    #[test]
    fn rejects_directory_with_wrong_attributes() {
        assert_eq!(
            check_dir_metadata(true, false, 501, 0o700),
            Err(SetupError::Symlink)
        );
        assert_eq!(
            check_dir_metadata(false, false, 501, 0o700),
            Err(SetupError::WrongType)
        );
        assert_eq!(
            check_dir_metadata(false, true, 0, 0o700),
            Err(SetupError::WrongOwner)
        );
        for mode in [0o755, 0o770, 0o701, 0o1700, 0o600] {
            assert_eq!(
                check_dir_metadata(false, true, 501, mode),
                Err(SetupError::WrongMode(mode)),
                "{mode:o}"
            );
        }
        assert_eq!(check_dir_metadata(false, true, 501, 0o40700), Ok(()));
    }

    #[test]
    fn rejects_symlinked_or_open_directory() {
        let base = private_dir();
        let target = base.path().join("real");
        fs::DirBuilder::new().mode(0o700).create(&target).unwrap();
        let link = base.path().join("link");
        symlink(&target, &link).unwrap();
        assert_eq!(check_dir(&link), Err(SetupError::Symlink));

        let open = base.path().join("open");
        fs::DirBuilder::new().mode(0o755).create(&open).unwrap();
        fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(check_dir(&open), Err(SetupError::WrongMode(0o755)));

        assert!(matches!(
            check_dir(&base.path().join("missing")),
            Err(SetupError::Io(io::ErrorKind::NotFound))
        ));
    }

    #[test]
    fn takes_token_once() {
        let dir = private_dir();
        let owner = check_dir(dir.path()).unwrap();
        let path = dir.path().join("token");
        let token = "0123456789abcdef".repeat(4);
        write_token(&path, &token, 0o600);
        assert_eq!(take_token(&path, owner), Ok(token));
        assert!(!path.exists(), "token file must be removed");
        assert!(matches!(
            take_token(&path, owner),
            Err(SetupError::Io(io::ErrorKind::NotFound))
        ));
    }

    #[test]
    fn rejects_bad_token_files() {
        let dir = private_dir();
        let owner = check_dir(dir.path()).unwrap();
        let path = dir.path().join("token");
        let token = "a".repeat(64);

        write_token(&path, &token, 0o644);
        assert_eq!(take_token(&path, owner), Err(SetupError::WrongMode(0o644)));
        assert!(!path.exists(), "rejected token is removed too");

        write_token(&path, "short", 0o600);
        assert_eq!(take_token(&path, owner), Err(SetupError::InvalidToken));

        write_token(&path, &"z".repeat(64), 0o600);
        assert_eq!(take_token(&path, owner), Err(SetupError::InvalidToken));

        write_token(&path, &"a".repeat(200), 0o600);
        assert_eq!(take_token(&path, owner), Err(SetupError::InvalidToken));

        // 別のユーザーのものとして扱う
        write_token(&path, &token, 0o600);
        let other = Owner {
            uid: owner.uid + 1,
            gid: owner.gid,
        };
        assert_eq!(take_token(&path, other), Err(SetupError::WrongOwner));

        // シンボリックリンクはたどらない
        let real = dir.path().join("real");
        write_token(&real, &token, 0o600);
        symlink(&real, &path).unwrap();
        assert_eq!(take_token(&path, owner), Err(SetupError::Symlink));
        assert!(real.exists());

        // ディレクトリ
        fs::remove_file(&path).unwrap();
        fs::DirBuilder::new().mode(0o600).create(&path).unwrap();
        assert!(take_token(&path, owner).is_err());
    }

    #[test]
    fn socket_is_private_and_reports_peer() {
        let dir = private_dir();
        let owner = check_dir(dir.path()).unwrap();
        let path = dir.path().join("helper.sock");
        let listener = bind_socket(&path, owner).unwrap();
        let meta = fs::symlink_metadata(&path).unwrap();
        assert_eq!(meta.mode() & 0o777, 0o600);
        assert_eq!(meta.uid(), owner.uid);

        let client = UnixStream::connect(&path).unwrap();
        let (server, _) = listener.accept().unwrap();
        let peer = peer_ids(&server).unwrap();
        assert_eq!(peer, peer_ids(&client).unwrap());
        assert!(peer_allowed(peer, owner));
        assert!(!peer_allowed(
            Owner {
                uid: owner.uid + 1,
                gid: owner.gid
            },
            owner
        ));
        assert!(!peer_allowed(Owner { uid: 0, gid: 0 }, owner));
    }
}
