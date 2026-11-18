//! ディスクの変化の検知（imple 6章）。DiskArbitrationのappeared・disappeared・description changedを
//! 専用スレッドのCFRunLoopで受け、最後のイベントから500msたったら1回だけ`on_change`を呼ぶ（間引き）。
//!
//! 登録した直後に、DiskArbitrationは既存のすべてのディスクについてappearedを送ってくる。
//! これで再取得すると起動直後の一覧の取得と重なるため、登録の直後の短い間のイベントは数えない。

mod debounce;

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use objc2_core_foundation::{CFArray, CFRunLoop, kCFRunLoopDefaultMode};
use objc2_disk_arbitration::{
    DADisk, DARegisterDiskAppearedCallback, DARegisterDiskDescriptionChangedCallback,
    DARegisterDiskDisappearedCallback, DASession, DAUnregisterCallback,
};

pub use debounce::debounce_loop;

/// 最後のイベントから再取得までの間（間引き）
pub const DEBOUNCE: Duration = Duration::from_millis(500);
/// 登録の直後に、既存のディスクの通知を読み捨てる時間
const PRIMING: Duration = Duration::from_millis(300);
/// 停止の指示を確かめる間隔
const STOP_POLL: Duration = Duration::from_millis(250);

type OnChange = Arc<dyn Fn() + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MonitorError {
    #[error("could not create a DiskArbitration session")]
    SessionUnavailable,
}

pub struct DeviceMonitor {
    on_change: OnChange,
    running: Mutex<Option<Running>>,
}

struct Running {
    stop: Arc<AtomicBool>,
    run_loop_thread: JoinHandle<()>,
    debounce_thread: JoinHandle<()>,
}

impl DeviceMonitor {
    /// `on_change`は、ディスクの変化が落ち着いたあとに、間引きのスレッドから呼ばれる
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            on_change: Arc::new(on_change),
            running: Mutex::new(None),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.lock().expect("monitor lock").is_some()
    }

    /// 検知を始める。すでに動いていれば何もしない
    pub fn start(&self) -> Result<(), MonitorError> {
        let mut running = self.running.lock().expect("monitor lock");
        if running.is_some() {
            return Ok(());
        }
        let stop = Arc::new(AtomicBool::new(false));
        let (events_tx, events_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), MonitorError>>();

        let run_loop_stop = Arc::clone(&stop);
        let run_loop_thread = std::thread::Builder::new()
            .name("device-monitor".into())
            .spawn(move || run_session(&events_tx, &run_loop_stop, &ready_tx))
            .expect("spawn device-monitor thread");
        if let Err(error) = ready_rx
            .recv()
            .unwrap_or(Err(MonitorError::SessionUnavailable))
        {
            let _ = run_loop_thread.join();
            return Err(error);
        }

        let on_change = Arc::clone(&self.on_change);
        let debounce_thread = std::thread::Builder::new()
            .name("device-monitor-debounce".into())
            .spawn(move || debounce_loop(&events_rx, DEBOUNCE, || on_change()))
            .expect("spawn device-monitor-debounce thread");

        *running = Some(Running {
            stop,
            run_loop_thread,
            debounce_thread,
        });
        Ok(())
    }

    /// 検知をやめる（コールバックの登録を外す）。動いていなければ何もしない
    pub fn stop(&self) {
        let Some(running) = self.running.lock().expect("monitor lock").take() else {
            return;
        };
        running.stop.store(true, Ordering::SeqCst);
        let _ = running.run_loop_thread.join();
        // CFRunLoopのスレッドが終わるとイベントの送り手がなくなり、間引きのスレッドも終わる
        let _ = running.debounce_thread.join();
    }
}

/// 監視の開始・停止（設定の`auto_refresh`につなぐ。テストでは偽実装を渡す）
pub trait MonitorControl: Send + Sync {
    fn is_running(&self) -> bool;
    fn start(&self) -> Result<(), MonitorError>;
    fn stop(&self);
}

impl MonitorControl for DeviceMonitor {
    fn is_running(&self) -> bool {
        DeviceMonitor::is_running(self)
    }

    fn start(&self) -> Result<(), MonitorError> {
        DeviceMonitor::start(self)
    }

    fn stop(&self) {
        DeviceMonitor::stop(self);
    }
}

/// 設定の`auto_refresh`に合わせて監視を開始・停止する。新しく開始したら`true`
/// （止めていたあいだの変化を反映するため、呼び出し側で1回再取得する）
pub fn apply_auto_refresh(
    monitor: &dyn MonitorControl,
    enabled: bool,
) -> Result<bool, MonitorError> {
    match (enabled, monitor.is_running()) {
        (true, false) => monitor.start().map(|()| true),
        (false, true) => {
            monitor.stop();
            Ok(false)
        }
        _ => Ok(false),
    }
}

impl Drop for DeviceMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// コールバックに渡す文脈
struct CallbackContext {
    events: Sender<()>,
    priming: AtomicBool,
}

impl CallbackContext {
    fn notify(&self) {
        if !self.priming.load(Ordering::SeqCst) {
            let _ = self.events.send(());
        }
    }
}

unsafe extern "C-unwind" fn disk_appeared(_disk: NonNull<DADisk>, context: *mut c_void) {
    // SAFETY: contextは`run_session`が登録したCallbackContextで、登録を外すまで有効
    unsafe { &*context.cast::<CallbackContext>() }.notify();
}

unsafe extern "C-unwind" fn disk_disappeared(_disk: NonNull<DADisk>, context: *mut c_void) {
    // SAFETY: 同上
    unsafe { &*context.cast::<CallbackContext>() }.notify();
}

unsafe extern "C-unwind" fn disk_description_changed(
    _disk: NonNull<DADisk>,
    _keys: NonNull<CFArray>,
    context: *mut c_void,
) {
    // SAFETY: 同上
    unsafe { &*context.cast::<CallbackContext>() }.notify();
}

/// 専用スレッドの本体。セッションを作ってコールバックを登録し、停止の指示までCFRunLoopを回す
fn run_session(events: &Sender<()>, stop: &AtomicBool, ready: &Sender<Result<(), MonitorError>>) {
    // SAFETY: 既定のアロケータでセッションを作る
    let Some(session) = (unsafe { DASession::new(None) }) else {
        let _ = ready.send(Err(MonitorError::SessionUnavailable));
        return;
    };
    let Some(run_loop) = CFRunLoop::current() else {
        let _ = ready.send(Err(MonitorError::SessionUnavailable));
        return;
    };
    // SAFETY: kCFRunLoopDefaultModeはCoreFoundationが持つ定数
    let mode = unsafe { kCFRunLoopDefaultMode };

    let context = Box::into_raw(Box::new(CallbackContext {
        events: events.clone(),
        priming: AtomicBool::new(true),
    }));
    let context_ptr = context.cast::<c_void>();
    // SAFETY: コールバックは上で定義した関数、contextは登録を外すまで解放しない。
    // matchにNULLを渡し、すべてのディスクを対象にする
    unsafe {
        DARegisterDiskAppearedCallback(&session, None, Some(disk_appeared), context_ptr);
        DARegisterDiskDisappearedCallback(&session, None, Some(disk_disappeared), context_ptr);
        DARegisterDiskDescriptionChangedCallback(
            &session,
            None,
            None,
            Some(disk_description_changed),
            context_ptr,
        );
        session.schedule_with_run_loop(&run_loop, mode.expect("default run loop mode"));
    }

    // 既存のディスクの通知を読み捨てる
    CFRunLoop::run_in_mode(mode, PRIMING.as_secs_f64(), false);
    // SAFETY: contextはまだ解放していない
    unsafe { &*context }.priming.store(false, Ordering::SeqCst);
    let _ = ready.send(Ok(()));

    while !stop.load(Ordering::SeqCst) {
        CFRunLoop::run_in_mode(mode, STOP_POLL.as_secs_f64(), false);
    }

    // SAFETY: 登録したのと同じセッション・関数・contextで外す。外したあとはコールバックが呼ばれない
    unsafe {
        session.unschedule_from_run_loop(&run_loop, mode.expect("default run loop mode"));
        for callback in [
            disk_appeared as *const c_void,
            disk_disappeared as *const c_void,
            disk_description_changed as *const c_void,
        ] {
            DAUnregisterCallback(
                &session,
                NonNull::new(callback.cast_mut()).expect("function pointer"),
                context_ptr,
            );
        }
        drop(Box::from_raw(context));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[derive(Default)]
    struct FakeMonitor {
        running: AtomicBool,
        starts: AtomicUsize,
        stops: AtomicUsize,
    }

    impl MonitorControl for FakeMonitor {
        fn is_running(&self) -> bool {
            self.running.load(Ordering::SeqCst)
        }
        fn start(&self) -> Result<(), MonitorError> {
            self.starts.fetch_add(1, Ordering::SeqCst);
            self.running.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn stop(&self) {
            self.stops.fetch_add(1, Ordering::SeqCst);
            self.running.store(false, Ordering::SeqCst);
        }
    }

    #[test]
    fn auto_refresh_starts_and_stops_only_on_change() {
        let monitor = FakeMonitor::default();
        assert!(apply_auto_refresh(&monitor, true).unwrap());
        assert!(!apply_auto_refresh(&monitor, true).unwrap());
        assert_eq!(monitor.starts.load(Ordering::SeqCst), 1);
        assert!(!apply_auto_refresh(&monitor, false).unwrap());
        assert!(!apply_auto_refresh(&monitor, false).unwrap());
        assert_eq!(monitor.stops.load(Ordering::SeqCst), 1);
        assert!(!monitor.is_running());
    }

    #[test]
    fn starts_and_stops_without_reporting_existing_disks() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let monitor = DeviceMonitor::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        assert!(!monitor.is_running());
        monitor.start().unwrap();
        monitor.start().unwrap();
        assert!(monitor.is_running());
        // 既存のディスクの通知は数えないため、ディスクに変化がなければ呼ばれない
        std::thread::sleep(DEBOUNCE + Duration::from_millis(200));
        monitor.stop();
        monitor.stop();
        assert!(!monitor.is_running());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        // 止めたあとも、もう一度始められる
        monitor.start().unwrap();
        monitor.stop();
    }
}
