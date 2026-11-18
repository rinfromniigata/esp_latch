#!/usr/bin/env bash
# 補助プロセス esp-latch-helper をビルドし、Tauriのサイドカー（bundle.externalBin）の命名で src-tauri/binaries/ に置く。
#
#   scripts/build_helper.sh          aarch64・x86_64をそれぞれビルドし、lipoでユニバーサルにする（tauri buildの前）
#   scripts/build_helper.sh --host   このMacのアーキテクチャだけをビルドする（tauri dev・checkコマンドの前）
#
# tauri-buildは、ビルドするアーキテクチャのターゲットトリプルを付けた名前
# （esp-latch-helper-aarch64-apple-darwin など）を要求し、Universalのバンドルは
# esp-latch-helper-universal-apple-darwin を同梱する。ファイルがないと本体のビルドが失敗する。
set -euo pipefail

cd "$(dirname "$0")/.."

NAME=esp-latch-helper
OUT=src-tauri/binaries
# 本体の最低OS（tauri.conf.jsonのbundle.macOS.minimumSystemVersion）にそろえる
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

build() {
	local triple=$1
	cargo build --release --quiet -p esp_latch_helper --target "$triple"
	install -m 0755 "target/$triple/release/$NAME" "$OUT/$NAME-$triple"
	echo "$OUT/$NAME-$triple"
}

mkdir -p "$OUT"
case "${1:-}" in
	--host)
		build "$(rustc -vV | sed -n 's/^host: //p')"
		;;
	"")
		build aarch64-apple-darwin
		build x86_64-apple-darwin
		lipo -create -output "$OUT/$NAME-universal-apple-darwin" \
			"$OUT/$NAME-aarch64-apple-darwin" "$OUT/$NAME-x86_64-apple-darwin"
		lipo -info "$OUT/$NAME-universal-apple-darwin"
		;;
	*)
		echo "usage: $0 [--host]" >&2
		exit 2
		;;
esac
