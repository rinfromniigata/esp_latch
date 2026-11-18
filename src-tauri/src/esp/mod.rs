//! マウント中のESPの中身（imple 6章「EspInspector」）：ブートローダーとその版、空き容量、ゴミ箱（`.Trashes`）の量。
//!
//! 一般ユーザーの権限で、マウント先のファイルを読むだけ。読めない値は`None`にし、UIは「不明」を出す。
//!
//! 版の読み方（T5-4で実物のバイナリを調べて確定）：
//! - Clover：`CLOVERX64.efi`に`Clover revision: 5176 (HEAD, commit …)`の文字列がある → `r5176`
//! - OpenCore：配布版の`OpenCore.efi`（1.0.8のRELEASE・DEBUG）には`REL-XXX-YYYY-MM-DD`の雛形しかなく、
//!   版は起動時に組み立てられる。ファイルに`REL-108-2026-09-28`の形が書かれていればそれを読み、
//!   なければ、起動に使ったESPに限ってOpenCoreがNVRAMに書いた`opencore-version`を使う（呼び出し側が渡す）

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::domain::{BootLoader, EspUsage, LoaderName};

const OPENCORE_EFI: &str = "EFI/OC/OpenCore.efi";
const CLOVER_EFI: &str = "EFI/CLOVER/CLOVERX64.efi";
const TRASHES: &str = ".Trashes";
const CLOVER_MARKER: &[u8] = b"Clover revision: ";
/// 版の文字列を探すために読む大きさの上限（実物は0.6〜1.6MB）
const MAX_LOADER_SIZE: u64 = 16 * 1024 * 1024;

/// ESPの中身の読み取り。テストでは偽実装を渡す
pub trait EspInspect: Send + Sync {
    /// `opencore_version`は、起動に使ったESPのときだけ渡すNVRAMの`opencore-version`（`REL-108-2026-09-28`）
    fn loader(&self, mount_point: &Path, opencore_version: Option<&str>) -> Option<BootLoader>;
    fn usage(&self, mount_point: &Path) -> EspUsage;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EspInspector;

impl EspInspect for EspInspector {
    fn loader(&self, mount_point: &Path, opencore_version: Option<&str>) -> Option<BootLoader> {
        if let Some(bytes) = read_loader(&mount_point.join(OPENCORE_EFI)) {
            let version = bytes
                .as_deref()
                .and_then(opencore_version_in)
                .or_else(|| opencore_version.and_then(parse_opencore_build));
            return Some(BootLoader {
                name: LoaderName::Opencore,
                version,
            });
        }
        if let Some(bytes) = read_loader(&mount_point.join(CLOVER_EFI)) {
            return Some(BootLoader {
                name: LoaderName::Clover,
                version: bytes.as_deref().and_then(clover_version_in),
            });
        }
        None
    }

    fn usage(&self, mount_point: &Path) -> EspUsage {
        let Ok((total, free)) = space(mount_point) else {
            // マウント先を読めない（取り外された直後など）
            return EspUsage {
                total: None,
                free: None,
                trash: None,
            };
        };
        let (total, free) = (Some(total), Some(free));
        let trash = match fs::symlink_metadata(mount_point.join(TRASHES)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Some(0),
            Err(_) => None,
            Ok(meta) if meta.is_dir() => allocated_size(&mount_point.join(TRASHES)).ok(),
            // シンボリックリンク・ファイルはゴミ箱として扱わない
            Ok(_) => None,
        };
        EspUsage { total, free, trash }
    }
}

/// ブートローダーのファイルを読む。ファイルがなければ`None`、あるが読めなければ`Some(None)`
fn read_loader(path: &Path) -> Option<Option<Vec<u8>>> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_LOADER_SIZE {
        return Some(None);
    }
    Some(fs::read(path).ok())
}

/// `OpenCore.efi`の中の`REL-108-2026-09-28`（`DBG-`・`NPT-`も）から`1.0.8`を得る。雛形（`REL-XXX-…`）は`None`
pub fn opencore_version_in(bytes: &[u8]) -> Option<String> {
    bytes.windows(18).find_map(|window| {
        std::str::from_utf8(window)
            .ok()
            .and_then(parse_opencore_build)
    })
}

/// OpenCoreのビルドの文字列（`REL-108-2026-09-28`）から版（`1.0.8`）を得る
pub fn parse_opencore_build(text: &str) -> Option<String> {
    let text = text.trim().trim_end_matches('\0');
    let bytes = text.as_bytes();
    let target = text.get(..3)?;
    if !matches!(target, "REL" | "DBG" | "NPT") || bytes.len() != 18 {
        return None;
    }
    let digits_at = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    let shape = bytes[3] == b'-'
        && digits_at(4..7)
        && bytes[7] == b'-'
        && digits_at(8..12)
        && bytes[12] == b'-'
        && digits_at(13..15)
        && bytes[15] == b'-'
        && digits_at(16..18);
    shape.then(|| {
        format!(
            "{}.{}.{}",
            bytes[4] as char, bytes[5] as char, bytes[6] as char
        )
    })
}

/// `CLOVERX64.efi`の中の`Clover revision: 5176`から`r5176`を得る
pub fn clover_version_in(bytes: &[u8]) -> Option<String> {
    let start = bytes
        .windows(CLOVER_MARKER.len())
        .position(|window| window == CLOVER_MARKER)?
        + CLOVER_MARKER.len();
    let digits: String = bytes[start..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .map(|&b| b as char)
        .collect();
    (!digits.is_empty() && digits.len() <= 6).then(|| format!("r{digits}"))
}

/// 全体の大きさと、一般ユーザーが使える空き容量（バイト数）
fn space(path: &Path) -> io::Result<(u64, u64)> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: NUL終端のパスと、十分な大きさの書き込み先を渡す
    if unsafe { libc::statfs(c_path.as_ptr(), stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: statfsが成功したので初期化済み
    let stat = unsafe { stat.assume_init() };
    let block = u64::from(stat.f_bsize);
    Ok((
        stat.f_blocks.saturating_mul(block),
        stat.f_bavail.saturating_mul(block),
    ))
}

/// ディレクトリ以下が占める大きさ（割り当てられたブロックの合計。ディレクトリ自身を含む）。
/// シンボリックリンクはたどらない。読めないものがあればエラー
fn allocated_size(path: &Path) -> io::Result<u64> {
    let meta = fs::symlink_metadata(path)?;
    let mut total = meta.blocks().saturating_mul(512);
    if meta.is_dir() {
        for entry in fs::read_dir(path)? {
            total = total.saturating_add(allocated_size(&entry?.path())?);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/bootloader")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn distributed_opencore_has_only_the_template() {
        // 配布版の1.0.8は、ファイルに版を持たない
        assert_eq!(
            opencore_version_in(&fixture("opencore_1.0.8_release.bin")),
            None
        );
        assert_eq!(
            opencore_version_in(&fixture("opencore_1.0.8_debug.bin")),
            None
        );
    }

    #[test]
    fn reads_opencore_build_strings() {
        assert_eq!(
            parse_opencore_build("REL-108-2026-09-28").as_deref(),
            Some("1.0.8")
        );
        assert_eq!(
            parse_opencore_build("DBG-107-2025-12-01\0").as_deref(),
            Some("1.0.7")
        );
        assert_eq!(
            parse_opencore_build("NPT-100-2024-04-01").as_deref(),
            Some("1.0.0")
        );
        for bad in [
            "REL-XXX-YYYY-MM-DD",
            "REL-10-2026-09-28",
            "ABC-108-2026-09-28",
            "REL-108-2026-09-2",
            "",
        ] {
            assert_eq!(parse_opencore_build(bad), None, "{bad}");
        }
        // ファイルに書かれていれば読む
        let mut bytes = fixture("opencore_1.0.8_release.bin");
        bytes.extend_from_slice(b"\0REL-106-2025-10-06\0");
        assert_eq!(opencore_version_in(&bytes).as_deref(), Some("1.0.6"));
    }

    #[test]
    fn reads_clover_revision() {
        assert_eq!(
            clover_version_in(&fixture("clover_5176.bin")).as_deref(),
            Some("r5176")
        );
        assert_eq!(clover_version_in(b"Clover revision: (HEAD)"), None);
        assert_eq!(clover_version_in(b"no marker"), None);
    }

    fn esp_with(path: &str, bytes: &[u8]) -> tempfile::TempDir {
        let volume = tempfile::tempdir().unwrap();
        let file = volume.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, bytes).unwrap();
        volume
    }

    #[test]
    fn detects_opencore_and_uses_the_nvram_version_for_the_boot_esp() {
        let volume = esp_with(OPENCORE_EFI, &fixture("opencore_1.0.8_release.bin"));
        let loader = EspInspector.loader(volume.path(), None).unwrap();
        assert_eq!(loader.name, LoaderName::Opencore);
        assert_eq!(loader.version, None);
        let loader = EspInspector
            .loader(volume.path(), Some("REL-108-2026-09-28"))
            .unwrap();
        assert_eq!(loader.version.as_deref(), Some("1.0.8"));
    }

    #[test]
    fn detects_clover() {
        let volume = esp_with(CLOVER_EFI, &fixture("clover_5176.bin"));
        let loader = EspInspector
            .loader(volume.path(), Some("REL-108-2026-09-28"))
            .unwrap();
        assert_eq!(loader.name, LoaderName::Clover);
        assert_eq!(loader.version.as_deref(), Some("r5176"));
    }

    #[test]
    fn no_loader_and_unreadable_loader() {
        let volume = tempfile::tempdir().unwrap();
        assert_eq!(EspInspector.loader(volume.path(), None), None);

        let volume = esp_with(OPENCORE_EFI, b"REL-108-2026-09-28");
        let file = volume.path().join(OPENCORE_EFI);
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
        let loader = EspInspector.loader(volume.path(), None).unwrap();
        assert_eq!(loader.name, LoaderName::Opencore);
        assert_eq!(loader.version, None, "unreadable file has no version");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn measures_space_and_trash() {
        let volume = tempfile::tempdir().unwrap();
        let usage = EspInspector.usage(volume.path());
        assert!(usage.total.unwrap() >= usage.free.unwrap());
        assert_eq!(usage.trash, Some(0), "no .Trashes means empty");

        let trash = volume.path().join(".Trashes/501/old EFI");
        fs::create_dir_all(&trash).unwrap();
        fs::write(trash.join("a.efi"), vec![1u8; 10_000]).unwrap();
        fs::write(volume.path().join("kept.efi"), vec![1u8; 50_000]).unwrap();
        let measured = EspInspector.usage(volume.path()).trash.unwrap();
        assert!((10_000..50_000).contains(&measured), "{measured}");
    }

    #[test]
    fn trash_is_unknown_when_unreadable_or_not_a_directory() {
        let volume = tempfile::tempdir().unwrap();
        let user = volume.path().join(".Trashes/501");
        fs::create_dir_all(&user).unwrap();
        fs::set_permissions(&user, fs::Permissions::from_mode(0o000)).unwrap();
        assert_eq!(EspInspector.usage(volume.path()).trash, None);
        fs::set_permissions(&user, fs::Permissions::from_mode(0o755)).unwrap();

        let volume = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), volume.path().join(".Trashes")).unwrap();
        assert_eq!(EspInspector.usage(volume.path()).trash, None);
    }

    #[test]
    fn missing_mount_point_has_unknown_values() {
        let usage = EspInspector.usage(Path::new("/nonexistent/esp-latch"));
        assert_eq!(
            usage,
            EspUsage {
                total: None,
                free: None,
                trash: None
            }
        );
    }
}
