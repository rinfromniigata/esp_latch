//! 操作ログに出すコマンドの文字列。本体と補助プロセスで同じ表記にする。

use std::path::Path;

/// プログラムはファイル名だけにし、空白などを含む引数は単引用符で囲む（`diskutil info -plist disk2s1`）
pub fn display_command(program: &str, args: &[&str]) -> String {
    let name = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    std::iter::once(name.to_owned())
        .chain(args.iter().map(|arg| shell_quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// シェルの単引用符でくくる。安全な文字だけの引数はそのまま返す
pub fn shell_quote(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=,+@%".contains(c));
    if plain {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_file_name_and_quotes() {
        assert_eq!(
            display_command("/usr/sbin/diskutil", &["info", "-plist", "disk2s1"]),
            "diskutil info -plist disk2s1"
        );
        assert_eq!(
            display_command("/usr/bin/open", &["/Volumes/EFI 1"]),
            "open '/Volumes/EFI 1'"
        );
        assert_eq!(display_command("echo", &["it's"]), r"echo 'it'\''s'");
        assert_eq!(display_command("echo", &[""]), "echo ''");
    }
}
