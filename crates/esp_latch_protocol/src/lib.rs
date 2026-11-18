//! ESP Latch本体と補助プロセスが共有する、要求・応答の型と引数の検証（imple 5章）。
//!
//! 通信は、一時ディレクトリの中のUNIXドメインソケットで、改行区切りのJSONを1行ずつやりとりする。

pub mod bsd;
pub mod command;

use serde::{Deserialize, Serialize};

/// 本体が作る一時ディレクトリの中の、トークンのファイル名（補助プロセスが読んだら消す）
pub const TOKEN_FILE: &str = "token";
/// 補助プロセスが一時ディレクトリの中に作るソケットのファイル名
pub const SOCKET_FILE: &str = "helper.sock";
/// トークンの長さ（ランダムな32バイトを16進の文字にしたもの）
pub const TOKEN_LEN: usize = 64;
/// 1行（要求・応答）の長さの上限。これを超える行は読まずに接続を切る
pub const MAX_LINE: usize = 64 * 1024;

/// GPTのESPの種別のGUID
pub const ESP_TYPE_GUID: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";

/// `diskutil info -plist`の`Content`が、ESP（GPTの`EFI`・種別のGUID、MBRの0xEF）を示すか
pub fn is_esp_content(content: &str) -> bool {
    content == "EFI"
        || content.eq_ignore_ascii_case(ESP_TYPE_GUID)
        || content.eq_ignore_ascii_case("0xEF")
}

/// 許可リストの操作。これ以外の要求は、JSONとして読めないものとして断る
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    /// ESPをマウントする。`diskutil mount`が失敗したら`mount -t msdos`に切り替える
    MountEsp { part: String },
    /// ESPを取り外す
    UnmountEsp { part: String },
    /// デバイスごと取り出す
    EjectDisk { disk: String },
    /// マウント中のESPの`.Trashes`の中身を消す
    EmptyTrash { part: String },
    /// 補助プロセスを終了する
    Shutdown,
}

impl Op {
    /// 引数のBSD名の形式を確かめる。パーティションは`^disk[0-9]+s[0-9]+$`、ディスクは`^disk[0-9]+$`
    pub fn validate(&self) -> Result<(), HelperError> {
        let valid = match self {
            Self::MountEsp { part } | Self::UnmountEsp { part } | Self::EmptyTrash { part } => {
                bsd::is_partition(part)
            }
            Self::EjectDisk { disk } => bsd::is_disk(disk),
            Self::Shutdown => true,
        };
        if valid {
            Ok(())
        } else {
            Err(HelperError::InvalidTarget)
        }
    }
}

/// 本体から補助プロセスへの要求（1行のJSON）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// 起動時にファイルで渡したトークン
    pub token: String,
    /// 応答と対応づける番号
    pub id: u64,
    pub op: Op,
}

/// 補助プロセスが実行したコマンド。本体が操作ログに記録する
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    /// 表示用のコマンドの文字列（`diskutil mount disk4s1`）
    pub command: String,
    /// 終了コード。起動できなかった・シグナルで終わった場合は`None`
    pub status: Option<i32>,
    /// 標準エラー出力（前後の空白を除く）
    pub stderr: String,
}

impl CommandRecord {
    pub fn succeeded(&self) -> bool {
        self.status == Some(0)
    }
}

/// 補助プロセスから本体への応答（1行のJSON）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    /// 実行したコマンド（実行した順）
    #[serde(default)]
    pub commands: Vec<CommandRecord>,
    /// `empty_trash`で解放したバイト数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freed_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<HelperError>,
}

impl Response {
    pub fn success(id: u64, commands: Vec<CommandRecord>) -> Self {
        Self {
            id,
            ok: true,
            commands,
            freed_bytes: None,
            error: None,
        }
    }

    pub fn failure(id: u64, commands: Vec<CommandRecord>, error: HelperError) -> Self {
        Self {
            id,
            ok: false,
            commands,
            freed_bytes: None,
            error: Some(error),
        }
    }
}

/// 補助プロセスが断った・失敗した理由
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HelperError {
    /// 要求をJSONとして読めない、または許可リストにない操作
    BadRequest,
    /// トークンが一致しない
    InvalidToken,
    /// BSD名の形式が違う、または対象が見つからない
    InvalidTarget,
    /// 対象がESPではない
    NotEsp,
    /// 対象がマウントされていない
    NotMounted,
    /// コマンドが失敗した
    CommandFailed { command: String, stderr: String },
    /// ファイルの操作に失敗した
    Io { message: String },
}

/// 値を1行のJSON（改行つき）にする
pub fn encode_line<T: Serialize>(value: &T) -> Vec<u8> {
    let mut line = serde_json::to_vec(value).expect("protocol types serialize");
    line.push(b'\n');
    line
}

/// 2つのトークンを、一致しない位置によらず同じ時間で比べる
pub fn tokens_match(expected: &str, given: &str) -> bool {
    let (expected, given) = (expected.as_bytes(), given.as_bytes());
    if expected.len() != given.len() || expected.is_empty() {
        return false;
    }
    expected
        .iter()
        .zip(given)
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_as_one_line() {
        let request = Request {
            token: "ab".repeat(32),
            id: 7,
            op: Op::MountEsp {
                part: "disk4s1".into(),
            },
        };
        let line = encode_line(&request);
        assert_eq!(line.last(), Some(&b'\n'));
        assert_eq!(line.iter().filter(|&&b| b == b'\n').count(), 1);
        let text = std::str::from_utf8(&line).unwrap();
        assert!(text.contains(r#""op":{"kind":"mount_esp","part":"disk4s1"}"#));
        let parsed: Request = serde_json::from_slice(&line).unwrap();
        assert_eq!(parsed, request);
    }

    #[test]
    fn rejects_operations_outside_the_allow_list() {
        for line in [
            r#"{"token":"x","id":1,"op":{"kind":"run","command":"rm -rf /"}}"#,
            r#"{"token":"x","id":1,"op":{"kind":"mount_esp","part":"disk4s1","path":"/"}}"#,
            r#"{"token":"x","id":1,"op":{"kind":"empty_trash","path":"/Volumes/EFI"}}"#,
            r#"{"token":"x","id":1,"op":{"kind":"mount_esp"}}"#,
            r#"{"token":"x","id":1,"op":"shutdown"}"#,
            r#"{"id":1,"op":{"kind":"shutdown"}}"#,
            r#"{"token":"x","id":1,"op":{"kind":"shutdown"},"extra":1}"#,
        ] {
            assert!(serde_json::from_str::<Request>(line).is_err(), "{line}");
        }
    }

    #[test]
    fn validates_bsd_names() {
        let ok = [
            Op::MountEsp {
                part: "disk4s1".into(),
            },
            Op::UnmountEsp {
                part: "disk12s10".into(),
            },
            Op::EmptyTrash {
                part: "disk0s1".into(),
            },
            Op::EjectDisk {
                disk: "disk4".into(),
            },
            Op::Shutdown,
        ];
        for op in ok {
            assert_eq!(op.validate(), Ok(()), "{op:?}");
        }
        let bad = [
            Op::MountEsp {
                part: "disk4".into(),
            },
            Op::MountEsp {
                part: "/dev/disk4s1".into(),
            },
            Op::UnmountEsp {
                part: "disk4s1 ".into(),
            },
            Op::EmptyTrash {
                part: "../disk4s1".into(),
            },
            Op::EjectDisk {
                disk: "disk4s1".into(),
            },
            Op::EjectDisk {
                disk: "disk4; reboot".into(),
            },
        ];
        for op in bad {
            assert_eq!(op.validate(), Err(HelperError::InvalidTarget), "{op:?}");
        }
    }

    #[test]
    fn response_omits_empty_optional_fields() {
        let response = Response::success(
            3,
            vec![CommandRecord {
                command: "diskutil mount disk4s1".into(),
                status: Some(0),
                stderr: String::new(),
            }],
        );
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"id":3,"ok":true,"commands":[{"command":"diskutil mount disk4s1","status":0,"stderr":""}]}"#
        );
        let failure = Response::failure(4, vec![], HelperError::NotEsp);
        assert_eq!(
            serde_json::to_string(&failure).unwrap(),
            r#"{"id":4,"ok":false,"commands":[],"error":{"kind":"not_esp"}}"#
        );
    }

    #[test]
    fn recognizes_esp_content() {
        assert!(is_esp_content("EFI"));
        assert!(is_esp_content("c12a7328-f81f-11d2-ba4b-00a0c93ec93b"));
        assert!(is_esp_content("0xEF"));
        for other in [
            "Apple_APFS",
            "Microsoft Basic Data",
            "efi",
            "DOS_FAT_32",
            "",
        ] {
            assert!(!is_esp_content(other), "{other}");
        }
    }

    #[test]
    fn compares_tokens() {
        let token = "0123456789abcdef".repeat(4);
        assert!(tokens_match(&token, &token.clone()));
        assert!(!tokens_match(&token, &token[..63]));
        assert!(!tokens_match(&token, &"0".repeat(64)));
        assert!(!tokens_match("", ""));
    }
}
