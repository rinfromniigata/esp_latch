//! ESPのゴミ箱（マウント先直下の`.Trashes`）の中身を消す（imple 5章）。
//!
//! rootで動くため、パスを組み立ててたどらず、ディレクトリのfdから`openat`・`unlinkat`で消す。
//! `.Trashes`そのものがシンボリックリンクなら断り、中のシンボリックリンクはリンクだけを消す（リンク先はたどらない）。

use std::ffi::{CStr, CString};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub const TRASHES: &str = ".Trashes";

/// `mount_point/.Trashes`の中身を消す。`.Trashes`がなければ`Ok(false)`、消したら`Ok(true)`
pub fn empty_trashes(mount_point: &Path) -> io::Result<bool> {
    let root = open_path(mount_point)?;
    let trashes = match open_child_dir(&root, c".Trashes") {
        Ok(fd) => fd,
        Err(error) if error.raw_os_error() == Some(libc::ENOENT) => return Ok(false),
        // シンボリックリンクはO_NOFOLLOWでELOOPかENOTDIR（macOSはO_DIRECTORYと組み合わせるとENOTDIR）
        Err(error) if matches!(error.raw_os_error(), Some(libc::ELOOP | libc::ENOTDIR)) => {
            return Err(io::Error::other(
                ".Trashes is not a real directory (symbolic link or file)",
            ));
        }
        Err(error) => return Err(error),
    };
    remove_contents(&trashes)?;
    Ok(true)
}

fn open_path(path: &Path) -> io::Result<OwnedFd> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: NUL終端のパスを渡す。成功したら所有するfdを返す
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    owned(fd)
}

/// `dir`の中の`name`を、シンボリックリンクをたどらずにディレクトリとして開く
fn open_child_dir(dir: &OwnedFd, name: &CStr) -> io::Result<OwnedFd> {
    // SAFETY: 有効なディレクトリのfdとNUL終端の名前を渡す
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    owned(fd)
}

fn owned(fd: i32) -> io::Result<OwnedFd> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: 開いたばかりのfdで、ほかに所有者はいない
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

/// ディレクトリの中身を再帰的に消す（ディレクトリそのものは残す）
fn remove_contents(dir: &OwnedFd) -> io::Result<()> {
    for name in list_names(dir)? {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: 有効なfd・NUL終端の名前・書き込み先を渡す。シンボリックリンクはたどらない
        let result = unsafe {
            libc::fstatat(
                dir.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fstatatが成功したので初期化済み
        let mode = unsafe { stat.assume_init() }.st_mode;
        let flags = if mode & libc::S_IFMT == libc::S_IFDIR {
            let child = open_child_dir(dir, &name)?;
            remove_contents(&child)?;
            libc::AT_REMOVEDIR
        } else {
            0
        };
        // SAFETY: 有効なfdとNUL終端の名前を渡す
        if unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), flags) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// ディレクトリの中の名前（`.`と`..`を除く）
fn list_names(dir: &OwnedFd) -> io::Result<Vec<CString>> {
    // fdopendirはfdを引き取るため、複製を渡す
    // SAFETY: 有効なfdを複製する
    let copy = owned(unsafe { libc::dup(dir.as_raw_fd()) })?;
    // SAFETY: 複製したfdの所有をDIRに移す（closedirで閉じる）
    let stream = unsafe { libc::fdopendir(std::os::fd::IntoRawFd::into_raw_fd(copy)) };
    if stream.is_null() {
        return Err(io::Error::last_os_error());
    }
    // 複製したfdは読み取り位置を共有するため、先頭から読む
    // SAFETY: 開いたDIRを渡す
    unsafe { libc::rewinddir(stream) };
    let mut names = Vec::new();
    loop {
        // SAFETY: 開いたDIRを渡す。戻り値は次のreaddirまで有効
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            break;
        }
        // SAFETY: d_nameはNUL終端
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name != c"." && name != c".." {
            names.push(name.to_owned());
        }
    }
    // SAFETY: 開いたDIRを1回だけ閉じる
    unsafe { libc::closedir(stream) };
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    #[test]
    fn empties_nested_trash_and_keeps_other_files() {
        let volume = tempfile::tempdir().unwrap();
        let root = volume.path();
        fs::create_dir_all(root.join("EFI/OC")).unwrap();
        fs::write(root.join("EFI/OC/OpenCore.efi"), b"oc").unwrap();
        fs::write(root.join("._EFI"), b"apple double").unwrap();
        let user = root.join(".Trashes/501");
        fs::create_dir_all(user.join("EFI old/OC/Drivers")).unwrap();
        fs::write(user.join("EFI old/OC/Drivers/x.efi"), vec![0u8; 4096]).unwrap();
        fs::write(user.join("config.plist"), b"x").unwrap();

        assert!(empty_trashes(root).unwrap());
        assert!(root.join(".Trashes").is_dir(), ".Trashes itself stays");
        assert_eq!(fs::read_dir(root.join(".Trashes")).unwrap().count(), 0);
        assert!(root.join("EFI/OC/OpenCore.efi").exists());
        assert!(root.join("._EFI").exists());
    }

    #[test]
    fn missing_trash_is_not_an_error() {
        let volume = tempfile::tempdir().unwrap();
        assert!(!empty_trashes(volume.path()).unwrap());
    }

    #[test]
    fn refuses_symlinked_trash() {
        let volume = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("precious"), b"keep").unwrap();
        symlink(outside.path(), volume.path().join(".Trashes")).unwrap();
        let error = empty_trashes(volume.path()).unwrap_err();
        assert!(
            error.to_string().contains("not a real directory"),
            "{error}"
        );
        assert!(outside.path().join("precious").exists());
    }

    #[test]
    fn refuses_trash_that_is_a_file() {
        let volume = tempfile::tempdir().unwrap();
        fs::write(volume.path().join(".Trashes"), b"x").unwrap();
        assert!(empty_trashes(volume.path()).is_err());
        assert!(volume.path().join(".Trashes").is_file());
    }

    #[test]
    fn removes_links_inside_trash_without_following_them() {
        let volume = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(outside.path().join("dir")).unwrap();
        fs::write(outside.path().join("dir/precious"), b"keep").unwrap();
        fs::write(outside.path().join("file"), b"keep").unwrap();
        let trash = volume.path().join(".Trashes/501");
        fs::create_dir_all(&trash).unwrap();
        symlink(outside.path().join("dir"), trash.join("dir link")).unwrap();
        symlink(outside.path().join("file"), trash.join("file link")).unwrap();

        assert!(empty_trashes(volume.path()).unwrap());
        assert_eq!(
            fs::read_dir(volume.path().join(".Trashes"))
                .unwrap()
                .count(),
            0
        );
        assert!(outside.path().join("dir/precious").exists());
        assert!(outside.path().join("file").exists());
    }
}
