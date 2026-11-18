//! 開発用：アプリ層を通してデバイスの一覧を取得し、UIが受け取るJSONを標準出力に出す。
//! 操作ログとイベントは標準エラー出力に出す。
//!
//! `cargo run -p esp_latch --example dump_devices`
//!
//! `-- --watch`を付けると、続けてディスクの変化を検知し、再取得のたびに各デバイスの1行の要約を出す（Ctrl-Cで終了）。

use std::sync::Arc;

use esp_latch_lib::app::AppCore;
use esp_latch_lib::boot::NvramBootInfo;
use esp_latch_lib::disk_info::DiskutilAdapter;
use esp_latch_lib::events::{AppEvent, EventSink};
use esp_latch_lib::monitor::DeviceMonitor;
use esp_latch_lib::runner::{CommandRunner, SystemRunner};

struct StderrSink;

impl EventSink for StderrSink {
    fn emit(&self, event: AppEvent) {
        match event {
            AppEvent::LogAppended(entry) => {
                eprintln!("[log] {:?} {}", entry.result, entry.command);
            }
            AppEvent::DevicesChanged(snapshot) => {
                eprintln!("[event] devices-changed");
                for device in snapshot.devices {
                    let mounted: Vec<_> = device
                        .partitions
                        .iter()
                        .filter_map(|p| p.mount_point.as_ref().map(|m| format!("{}={m}", p.id)))
                        .collect();
                    eprintln!(
                        "  {} {:?} {:?} {mounted:?}",
                        device.id, device.kind, device.labels
                    );
                }
            }
            other => eprintln!("[event] {}", other.name()),
        }
    }
}

fn main() {
    let runner: Arc<dyn CommandRunner> = Arc::new(SystemRunner);
    let core = Arc::new(AppCore::new(
        Arc::new(DiskutilAdapter::new(Arc::clone(&runner))),
        Arc::new(NvramBootInfo::new(runner)),
        Arc::new(StderrSink),
    ));
    let started = std::time::Instant::now();
    match core.list_devices() {
        Ok(snapshot) => {
            eprintln!("[time] {:?}", started.elapsed());
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshot).expect("serialize")
            );
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
    if std::env::args().any(|arg| arg == "--watch") {
        let watched = Arc::clone(&core);
        let monitor = DeviceMonitor::new(move || {
            if let Err(error) = watched.refresh() {
                eprintln!("refresh failed: {error}");
            }
        });
        monitor.start().expect("start monitor");
        eprintln!("[watch] started");
        loop {
            std::thread::park();
        }
    }
}
