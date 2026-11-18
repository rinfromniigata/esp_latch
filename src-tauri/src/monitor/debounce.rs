//! 間引き：イベントが続くあいだは待ち、最後のイベントから`interval`たったら1回だけ`action`を呼ぶ。

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

/// 送り手がすべてなくなるまで続ける。待っている途中で送り手がなくなったら、`action`を呼ばずに終わる
pub fn debounce_loop(events: &Receiver<()>, interval: Duration, mut action: impl FnMut()) {
    while events.recv().is_ok() {
        loop {
            match events.recv_timeout(interval) {
                Ok(()) => {}
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        action();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    const INTERVAL: Duration = Duration::from_millis(60);

    fn run(schedule: &[u64]) -> Vec<Duration> {
        let (tx, rx) = mpsc::channel();
        let started = Instant::now();
        let worker = thread::spawn(move || {
            let mut calls = Vec::new();
            debounce_loop(&rx, INTERVAL, || calls.push(started.elapsed()));
            calls
        });
        for delay in schedule {
            thread::sleep(Duration::from_millis(*delay));
            tx.send(()).unwrap();
        }
        // 最後の待ちが終わるまで送り手を残す
        thread::sleep(INTERVAL * 3);
        drop(tx);
        worker.join().unwrap()
    }

    #[test]
    fn burst_of_events_runs_once_after_the_last() {
        let calls = run(&[0, 10, 10, 10, 10]);
        assert_eq!(calls.len(), 1);
        // 最後のイベント（40ms付近）から間引きの時間がたってから呼ばれる
        assert!(
            calls[0] >= Duration::from_millis(40) + INTERVAL,
            "{calls:?}"
        );
    }

    #[test]
    fn separated_events_run_separately() {
        let calls = run(&[0, 200]);
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn disconnect_while_waiting_skips_action() {
        let (tx, rx) = mpsc::channel();
        tx.send(()).unwrap();
        drop(tx);
        let mut called = false;
        debounce_loop(&rx, INTERVAL, || called = true);
        assert!(!called);
    }
}
