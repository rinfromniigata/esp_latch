//! 起動の引数：`--dir <一時ディレクトリ> --parent-pid <本体のPID>`。ほかの引数は受け付けない。

use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// 本体が作った一時ディレクトリ（絶対パス）
    pub dir: PathBuf,
    /// 本体のPID。終了したら補助プロセスも終了する
    pub parent_pid: i32,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
    let mut dir = None;
    let mut parent_pid = None;
    let mut args = args.into_iter();
    while let Some(name) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", name.to_string_lossy()))?;
        match name.to_str() {
            Some("--dir") if dir.is_none() => dir = Some(PathBuf::from(value)),
            Some("--parent-pid") if parent_pid.is_none() => {
                let pid = value
                    .to_str()
                    .and_then(|text| text.parse::<i32>().ok())
                    .filter(|&pid| pid > 1)
                    .ok_or("invalid --parent-pid")?;
                parent_pid = Some(pid);
            }
            _ => return Err(format!("unexpected argument {}", name.to_string_lossy())),
        }
    }
    let dir = dir.ok_or("missing --dir")?;
    if !dir.is_absolute() {
        return Err("--dir must be absolute".into());
    }
    Ok(Args {
        dir,
        parent_pid: parent_pid.ok_or("missing --parent-pid")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Args, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn parses_both_arguments_in_any_order() {
        let expected = Args {
            dir: "/private/var/folders/x/T/esp-latch-1".into(),
            parent_pid: 4242,
        };
        assert_eq!(
            parse_strs(&[
                "--dir",
                "/private/var/folders/x/T/esp-latch-1",
                "--parent-pid",
                "4242"
            ]),
            Ok(expected.clone())
        );
        assert_eq!(
            parse_strs(&[
                "--parent-pid",
                "4242",
                "--dir",
                "/private/var/folders/x/T/esp-latch-1"
            ]),
            Ok(expected)
        );
    }

    #[test]
    fn rejects_bad_arguments() {
        for args in [
            &[][..],
            &["--dir", "/tmp/x"],
            &["--parent-pid", "42"],
            &["--dir", "relative", "--parent-pid", "42"],
            &["--dir", "/tmp/x", "--parent-pid", "1"],
            &["--dir", "/tmp/x", "--parent-pid", "-5"],
            &["--dir", "/tmp/x", "--parent-pid", "abc"],
            &["--dir", "/tmp/x", "--parent-pid", "42", "--dir", "/tmp/y"],
            &["--dir", "/tmp/x", "--parent-pid", "42", "--extra", "1"],
            &["--dir"],
        ] {
            assert!(parse_strs(args).is_err(), "{args:?}");
        }
    }
}
