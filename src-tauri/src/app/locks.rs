//! 同じディスクへの操作を同時に1つだけにするロック。2つ目は待たずに`busy`で断る。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use esp_latch_protocol::bsd;

use super::AppError;

#[derive(Default)]
pub struct DiskLocks {
    held: Arc<Mutex<HashSet<String>>>,
}

impl DiskLocks {
    /// `target`（ディスクかパーティションのBSD名）が属するディスクのロックを取る。
    /// 形式が違えば`InvalidTarget`、同じディスクを操作中なら`Busy`
    pub fn try_lock(&self, target: &str) -> Result<DiskLock, AppError> {
        let disk = bsd::whole_disk(target).ok_or(AppError::InvalidTarget)?;
        let mut held = self.held.lock().expect("disk locks");
        if !held.insert(disk.to_owned()) {
            return Err(AppError::Busy);
        }
        Ok(DiskLock {
            disk: disk.to_owned(),
            held: Arc::clone(&self.held),
        })
    }
}

/// 取ったロック。破棄すると外れる
pub struct DiskLock {
    disk: String,
    held: Arc<Mutex<HashSet<String>>>,
}

impl DiskLock {
    pub fn disk(&self) -> &str {
        &self.disk
    }
}

impl Drop for DiskLock {
    fn drop(&mut self) {
        if let Ok(mut held) = self.held.lock() {
            held.remove(&self.disk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_operation_on_same_disk_is_busy() {
        let locks = DiskLocks::default();
        let first = locks.try_lock("disk4s1").unwrap();
        assert_eq!(first.disk(), "disk4");
        assert!(matches!(locks.try_lock("disk4"), Err(AppError::Busy)));
        assert!(matches!(locks.try_lock("disk4s2"), Err(AppError::Busy)));
        // 別のディスクは操作できる
        let _other = locks.try_lock("disk40s1").unwrap();
        drop(first);
        assert!(locks.try_lock("disk4").is_ok());
    }

    #[test]
    fn invalid_target_is_rejected() {
        let locks = DiskLocks::default();
        assert!(matches!(
            locks.try_lock("/dev/disk4"),
            Err(AppError::InvalidTarget)
        ));
        assert!(matches!(locks.try_lock(""), Err(AppError::InvalidTarget)));
    }
}
