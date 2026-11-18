#!/bin/bash
# 開発用のディスクイメージを作り、接続・切断する。内蔵ディスクに触れずにESP Latchを試すためのもの。
#
# 使い方: scripts/test_image.sh <create|attach|detach|status|clean> [gpt|broken|mbr|all]
#
#   gpt     ESP（FAT32・200MB）とAPFSコンテナ（ボリューム「ESP Latch Test」）を持つGPTのイメージ
#   broken  gptと同じ構成で、ESPのファイルシステムの先頭を壊したもの（マウントの失敗の確認用）
#   mbr     ESPのない、MBR・exFAT（ボリューム「ESPLATCHMBR」）のイメージ
#
# イメージは $ESP_LATCH_TEST_IMAGE_DIR（既定は target/test_images/）にスパースイメージで作る。
# 管理者権限は要らない（自分で接続したイメージのデバイスは、接続したユーザーが所有するため）。
# パーティションの作成・フォーマット・破壊は、このスクリプトが接続したディスクイメージの
# デバイスだけに行い、ほかのディスクには触れない。
#
# hdiutilのcreate・attachはmacOS 26以降で非推奨（diskutil imageへの移行を促す警告が出る）だが、
# 開発環境の最低OS（macOS 11）でも動くようにhdiutilを使い、警告は表示しない。

set -euo pipefail

ESP_TYPE_GUID="C12A7328-F81F-11D2-BA4B-00A0C93EC93B"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_DIR="${ESP_LATCH_TEST_IMAGE_DIR:-$REPO_ROOT/target/test_images}"
KINDS=(gpt broken mbr)

usage() {
	sed -n '4,9p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2
	exit 2
}

die() {
	echo "test_image.sh: $*" >&2
	exit 1
}

# hdiutilの非推奨の警告を除いて実行する
hdiutil_quiet() {
	hdiutil "$@" 2> >(grep -v 'is deprecated' >&2)
}

image_path() {
	echo "$IMAGE_DIR/$1.sparseimage"
}

# 種別の引数を、対象の種別の並びに展開する
expand_kinds() {
	case "${1:-}" in
	all) echo "${KINDS[@]}" ;;
	gpt | broken | mbr) echo "$1" ;;
	*) usage ;;
	esac
}

# 指定したイメージファイルを接続しているディスク（/dev/diskN）を返す。接続していなければ空
attached_device() {
	local path
	path="$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")"
	hdiutil info 2>/dev/null | awk -v path="$path" '
		/^image-path/ { sub(/^image-path[ \t]*:[ \t]*/, ""); current = $0; next }
		/^\/dev\/disk[0-9]+[ \t]/ && current == path { print $1; exit }
	'
}

# デバイスがディスクイメージであることを確かめる（イメージ以外を誤って書き換えないため）
assert_disk_image() {
	diskutil info -plist "$1" | plutil -extract BusProtocol xml1 -o - - |
		grep -q '<string>Disk Image</string>' ||
		die "$1 はディスクイメージではありません。中止します"
}

# イメージを作る。作るあいだだけ接続し、最後に切断する
create_one() {
	local kind="$1" path dev
	path="$(image_path "$kind")"
	[[ -e "$path" ]] && die "$path はすでにあります（作り直すときは clean $kind を先に実行してください）"
	mkdir -p "$IMAGE_DIR"

	hdiutil_quiet create -quiet -size 512m -type SPARSE -layout NONE -o "$path"
	dev="$(hdiutil_quiet attach -nomount "$path" | awk 'NR == 1 { print $1 }')"
	[[ "$dev" =~ ^/dev/disk[0-9]+$ ]] || die "$path を接続できませんでした"
	assert_disk_image "$dev"

	case "$kind" in
	gpt | broken)
		# 512MBではdiskutilがESPを自動で作らないため、ESPの種別GUIDで未フォーマットの区画を作り、FAT32にする
		diskutil partitionDisk "$dev" GPT "%$ESP_TYPE_GUID%" %noformat% 200M \
			APFS "ESP Latch $([[ $kind == gpt ]] && echo Test || echo Broken)" R >/dev/null
		newfs_msdos -F 32 -v EFI "/dev/r${dev#/dev/}s1" >/dev/null 2>&1
		if [[ "$kind" == broken ]]; then
			# ブートセクタとFATの先頭を0で埋め、マウントできない状態にする
			dd if=/dev/zero of="/dev/r${dev#/dev/}s1" bs=512 count=64 2>/dev/null
		fi
		;;
	mbr)
		diskutil partitionDisk "$dev" MBR ExFAT ESPLATCHMBR R >/dev/null
		;;
	esac

	hdiutil_quiet detach -quiet "$dev"
	echo "created  $kind  $path"
}

attach_one() {
	local kind="$1" path dev
	path="$(image_path "$kind")"
	[[ -e "$path" ]] || die "$path がありません（先に create $kind を実行してください）"
	dev="$(attached_device "$path")"
	if [[ -n "$dev" ]]; then
		echo "attached $kind  ${dev}（接続済み）"
		return
	fi
	# ESPはmacOSが自動ではマウントしない。APFS・exFATのボリュームはマウントされる
	dev="$(hdiutil_quiet attach "$path" | awk 'NR == 1 { print $1 }')"
	echo "attached $kind  $dev"
}

detach_one() {
	local kind="$1" dev
	dev="$(attached_device "$(image_path "$kind")")"
	[[ -n "$dev" ]] || return 0
	# ESPがマウントされたままでも切断できるよう、ディスクごと取り出す
	diskutil eject "$dev" >/dev/null || hdiutil_quiet detach -quiet -force "$dev"
	echo "detached $kind  $dev"
}

status_one() {
	local kind="$1" path dev
	path="$(image_path "$kind")"
	if [[ ! -e "$path" ]]; then
		echo "$kind: なし"
		return
	fi
	dev="$(attached_device "$path")"
	echo "$kind: ${dev:-未接続}  $path"
}

clean_one() {
	detach_one "$1"
	rm -f "$(image_path "$1")"
}

[[ $# -ge 1 ]] || usage
command="$1"
case "$command" in
create | attach | detach | clean) kinds="$(expand_kinds "${2:-}")" ;;
status) kinds="$(expand_kinds "${2:-all}")" ;;
*) usage ;;
esac

for kind in $kinds; do
	"${command}_one" "$kind"
done
