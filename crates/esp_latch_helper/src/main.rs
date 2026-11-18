//! ESP Latchの補助プロセス（root）。許可リストの操作だけを実行する（imple 5章）。
//!
//! 起動：`esp-latch-helper --dir <一時ディレクトリ> --parent-pid <本体のPID>`

fn main() {
    if let Err(error) = esp_latch_helper::run(std::env::args_os().skip(1)) {
        eprintln!("esp-latch-helper: {error}");
        std::process::exit(1);
    }
}
