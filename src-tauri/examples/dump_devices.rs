//! 開発用：アプリ層を通してデバイスの一覧を取得し、UIが受け取るJSONを標準出力に出す。
//! 操作ログとイベントは標準エラー出力に出す。
//!
//! `cargo run -p esp_latch --example dump_devices`
//!
//! `-- --watch`を付けると、続けてディスクの変化を検知し、再取得のたびに各デバイスの1行の要約を出す（Ctrl-Cで終了）。
//! `-- --inspect disk4s1`を付けると、マウント中のESPの詳細（`inspect_esp`の結果）を出す。特権操作は使わない。

use std::sync::Arc;

use esp_latch_lib::app::{AppCore, Ports};
use esp_latch_lib::boot::NvramBootInfo;
use esp_latch_lib::disk_info::DiskutilAdapter;
use esp_latch_lib::esp::EspInspector;
use esp_latch_lib::events::{AppEvent, EventSink};
use esp_latch_lib::monitor::DeviceMonitor;
use esp_latch_lib::privileged::{
    Op, PrivilegedError, PrivilegedErrorKind, PrivilegedOps, PrivilegedReply,
};
use esp_latch_lib::runner::{CommandRunner, SystemRunner};

/// 特権操作は使わない（認証のダイアログを出さない）
struct NoPrivileged;

impl PrivilegedOps for NoPrivileged {
    fn execute(&self, _: Op) -> Result<PrivilegedReply, PrivilegedError> {
        Err(PrivilegedError {
            kind: PrivilegedErrorKind::Unavailable {
                message: "not available in this example".into(),
            },
            launch: None,
        })
    }

    fn shutdown(&self) {}
}

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
        Ports {
            disk_info: Arc::new(DiskutilAdapter::new(Arc::clone(&runner))),
            boot_info: Arc::new(NvramBootInfo::new(Arc::clone(&runner))),
            privileged: Arc::new(NoPrivileged),
            inspector: Arc::new(EspInspector),
            runner,
        },
        Arc::new(StderrSink),
    ));
    let args: Vec<String> = std::env::args().collect();
    if let Some(part) = args
        .iter()
        .position(|arg| arg == "--inspect")
        .and_then(|index| args.get(index + 1))
    {
        match core.inspect_esp(part) {
            Ok(detail) => println!(
                "{}",
                serde_json::to_string_pretty(&detail).expect("serialize")
            ),
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
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
