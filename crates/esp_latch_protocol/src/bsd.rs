//! BSD名（`disk2`・`disk2s1`）の検証。UIから受け取った引数は、本体と補助プロセスの両方でこれを通す。

/// ディスク全体のBSD名（`^disk[0-9]+$`）か
pub fn is_disk(id: &str) -> bool {
    id.strip_prefix("disk").is_some_and(is_number)
}

/// パーティションのBSD名（`^disk[0-9]+s[0-9]+$`）か
pub fn is_partition(id: &str) -> bool {
    id.strip_prefix("disk")
        .and_then(|rest| rest.split_once('s'))
        .is_some_and(|(disk, slice)| is_number(disk) && is_number(slice))
}

/// パーティションまたはディスクのBSD名から、ディスク全体のBSD名を返す。形式が違えば`None`
pub fn whole_disk(id: &str) -> Option<&str> {
    if is_disk(id) {
        Some(id)
    } else if is_partition(id) {
        id.rfind('s').map(|index| &id[..index])
    } else {
        None
    }
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.len() <= 6 && text.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_disks_and_partitions() {
        assert!(is_disk("disk0"));
        assert!(is_disk("disk12"));
        assert!(is_partition("disk2s1"));
        assert!(is_partition("disk12s10"));
    }

    #[test]
    fn rejects_other_forms() {
        for id in [
            "",
            "disk",
            "disks1",
            "disk2s",
            "disk2s1s1",
            "/dev/disk2",
            "disk2 ",
            "disk-1",
            "disk２",
            "rdisk2",
            "disk2s1; rm -rf /",
            "disk1234567",
        ] {
            assert!(!is_disk(id), "{id:?}");
            assert!(!is_partition(id), "{id:?}");
        }
        assert!(!is_partition("disk2"));
        assert!(!is_disk("disk2s1"));
    }

    #[test]
    fn finds_whole_disk() {
        assert_eq!(whole_disk("disk2s1"), Some("disk2"));
        assert_eq!(whole_disk("disk12s10"), Some("disk12"));
        assert_eq!(whole_disk("disk4"), Some("disk4"));
        assert_eq!(whole_disk("disk3s1s1"), None);
        assert_eq!(whole_disk("../disk2"), None);
    }
}
