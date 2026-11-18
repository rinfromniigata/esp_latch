//! 起動に使ったESPの判定。OpenCoreがNVRAMに書く`boot-path`から、GPTのパーティションUUIDを取り出す。
//!
//! Cloverでの起動や実機のMacでは値がないため、印を出さない（スペック9章の決定 (b)）。

use std::sync::Arc;

use crate::runner::{CommandRunner, programs};

/// OpenCoreのベンダーGUIDの`boot-path`（例：`PciRoot(0x0)/…/HD(1,GPT,<UUID>,0x28,0x64000)/\EFI\OC\OpenCore.efi`）
const BOOT_PATH_VARIABLE: &str = "4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:boot-path";

/// 起動に使ったパーティションの情報
pub trait BootInfo: Send + Sync {
    /// 起動に使ったESPのGPTのパーティションUUID（大文字）。分からなければ`None`
    fn boot_part_uuid(&self) -> Option<String>;
}

pub struct NvramBootInfo {
    runner: Arc<dyn CommandRunner>,
}

impl NvramBootInfo {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl BootInfo for NvramBootInfo {
    fn boot_part_uuid(&self) -> Option<String> {
        // 値がないとき、nvramは「data was not found」で失敗する。それ以外の失敗も、印を出さないだけにする
        let output = self
            .runner
            .run_checked(programs::NVRAM, &[BOOT_PATH_VARIABLE])
            .ok()?;
        parse_boot_path(&String::from_utf8_lossy(&output.stdout))
    }
}

/// `nvram`の出力（`<変数名>\t<値>`）から、`GPT,`に続くUUIDを取り出す
fn parse_boot_path(output: &str) -> Option<String> {
    let value = output.split_once('\t').map_or(output, |(_, value)| value);
    value.match_indices("GPT,").find_map(|(index, marker)| {
        let candidate = value.get(index + marker.len()..index + marker.len() + 36)?;
        is_uuid(candidate).then(|| candidate.to_ascii_uppercase())
    })
}

fn is_uuid(text: &str) -> bool {
    text.len() == 36
        && text.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::FakeRunner;

    const COMMAND: &str = "nvram 4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:boot-path";

    fn boot_info(status: i32, stdout: &str, stderr: &str) -> NvramBootInfo {
        let runner = FakeRunner::new();
        runner.respond(COMMAND, status, stdout.as_bytes(), stderr);
        NvramBootInfo::new(Arc::new(runner))
    }

    #[test]
    fn extracts_gpt_uuid_from_sata_boot_path() {
        let info = boot_info(
            0,
            "4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:boot-path\tPciRoot(0x0)/Pci(0x17,0x0)/Sata(0x0,0xFFFF,0x0)/HD(1,GPT,6c5db8d5-6b5c-4c61-accb-c7c2f1e2b6b6,0x28,0x64000)/\\EFI\\OC\\OpenCore.efi\n",
            "",
        );
        assert_eq!(
            info.boot_part_uuid().as_deref(),
            Some("6C5DB8D5-6B5C-4C61-ACCB-C7C2F1E2B6B6")
        );
    }

    #[test]
    fn extracts_gpt_uuid_from_usb_and_nvme_paths() {
        let usb = "PciRoot(0x0)/Pci(0x14,0x0)/USB(0x2,0x0)/HD(1,GPT,0A1B2C3D-4E5F-6071-8293-A4B5C6D7E8F9,0x800,0x64000)/\\EFI\\BOOT\\BOOTx64.efi";
        assert_eq!(
            parse_boot_path(usb).as_deref(),
            Some("0A1B2C3D-4E5F-6071-8293-A4B5C6D7E8F9")
        );
        let nvme = "x:boot-path\tPciRoot(0x0)/Pci(0x1D,0x0)/Pci(0x0,0x0)/NVMe(0x1,00-00-00-00-00-00-00-00)/HD(1,GPT,11111111-2222-3333-4444-555555555555,0x28,0x64000)/\\EFI\\OC\\OpenCore.efi";
        assert_eq!(
            parse_boot_path(nvme).as_deref(),
            Some("11111111-2222-3333-4444-555555555555")
        );
    }

    #[test]
    fn missing_variable_is_none() {
        let info = boot_info(
            1,
            "",
            "nvram: Error getting variable - '4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:boot-path': (iokit/common) data was not found",
        );
        assert_eq!(info.boot_part_uuid(), None);
    }

    #[test]
    fn non_gpt_or_malformed_values_are_none() {
        // MBRのパーティション（署名は8桁の16進）
        assert_eq!(
            parse_boot_path(
                "PciRoot(0x0)/HD(1,MBR,0x12345678,0x800,0x64000)/\\EFI\\OC\\OpenCore.efi"
            ),
            None
        );
        assert_eq!(parse_boot_path("HD(1,GPT,not-a-uuid,0x28)"), None);
        assert_eq!(parse_boot_path(""), None);
    }

    #[test]
    fn runner_failure_is_none() {
        let info = NvramBootInfo::new(Arc::new(FakeRunner::new()));
        assert_eq!(info.boot_part_uuid(), None);
    }
}
