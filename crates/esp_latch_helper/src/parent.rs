//! 本体のPIDの監視。本体が終了したら（kqueueの`NOTE_EXIT`）、補助プロセスも終了する。

use std::io;
use std::ptr;

/// `pid`のプロセスが終了するまで待つ。すでに終了していれば、すぐに戻る
pub fn wait_for_exit(pid: i32) -> io::Result<()> {
    // SAFETY: 引数のないシステムコール
    let queue = unsafe { libc::kqueue() };
    if queue < 0 {
        return Err(io::Error::last_os_error());
    }
    let _close = CloseOnDrop(queue);
    let change = libc::kevent {
        ident: pid as libc::uintptr_t,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: ptr::null_mut(),
    };
    // SAFETY: 1件の変更を登録するだけで、受け取りはしない
    let result = unsafe { libc::kevent(queue, &change, 1, ptr::null_mut(), 0, ptr::null()) };
    if result < 0 {
        let error = io::Error::last_os_error();
        // 登録の前に終了していた
        return if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(())
        } else {
            Err(error)
        };
    }
    loop {
        // SAFETY: zeroedはkeventとして有効な値（整数とヌルポインタ）
        let mut event: libc::kevent = unsafe { std::mem::zeroed() };
        // SAFETY: 1件分の受け取り先を渡し、終了の通知まで待つ
        let count = unsafe { libc::kevent(queue, ptr::null(), 0, &mut event, 1, ptr::null()) };
        if count > 0 {
            return Ok(());
        }
        if count < 0 {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
}

/// プロセスが動いているか（`kill(pid, 0)`。権限がなくても、存在すれば動いているとみなす）
pub fn is_alive(pid: i32) -> bool {
    // SAFETY: シグナル0は送らずに存在だけを確かめる
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

struct CloseOnDrop(i32);

impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        // SAFETY: kqueueが返したfdを1回だけ閉じる
        unsafe { libc::close(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn returns_when_the_process_exits() {
        let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let pid = child.id() as i32;
        assert!(is_alive(pid));
        let (done, finished) = mpsc::channel();
        std::thread::spawn(move || done.send(wait_for_exit(pid)).unwrap());
        // 終了する前には戻らない
        assert!(finished.recv_timeout(Duration::from_millis(200)).is_err());
        child.kill().unwrap();
        let result = finished
            .recv_timeout(Duration::from_secs(5))
            .expect("woke up");
        assert!(result.is_ok());
        child.wait().unwrap();
        assert!(!is_alive(pid));
    }

    #[test]
    fn returns_immediately_for_a_finished_process() {
        let mut child = Command::new("/usr/bin/true").spawn().unwrap();
        let pid = child.id() as i32;
        child.wait().unwrap();
        assert!(wait_for_exit(pid).is_ok());
    }
}
