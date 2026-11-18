# esp_latch imple（アーキテクチャと設計）

正本：`roadmap/specs/esp_latch_SPEC_v1.md`（振る舞い・非機能・技術）、`esp_latch_tokens_v1.css`（値）、`esp_latch_mock_{画面ID}_v1.html`（画面構成）、Carbon Design System v11（部品の振る舞い）。このファイルは、それらをどう実装するかを定める。値・文言を写さない。

## 1. 全体の構成

```
UI層（Svelte 5＋SvelteKit SPA、3つのウィンドウ）
  └─ api/（invokeとイベントの型つきの窓口）── stores/（状態）── components/（Carbonの部品＋独自の部品）
        │ invoke / listen
アプリ層（Rust・Tauriコマンド。ユースケース単位）
  ├─ DiskInfo（trait）……… DiskutilAdapter（diskutil・hdiutilのplist → ドメインモデル）
  ├─ BootInfo（trait）……… NvramBootInfo（boot-pathのGPT UUID）
  ├─ EspInspector ………… ブートローダーの検出、空き容量と.Trashesの量
  ├─ DeviceMonitor ……… DiskArbitrationのイベント → 間引き → 再取得
  ├─ PrivilegedOps（trait）… HelperGateway（osascriptで補助プロセスを起動し、ソケットで要求）／FakePrivilegedOps（テスト用）
  ├─ OperationLog ……… メモリ内の操作ログ
  └─ SettingsStore ……… 設定ファイルの読み書き
補助プロセス esp-latch-helper（Rust・別バイナリ、root）
  └─ 許可リストの操作だけを、引数とパーティションを検証して実行する
共有クレート esp_latch_protocol
  └─ 要求・応答の型、BSD名の検証（本体と補助プロセスの両方が使う）
```

- 上位は下位のtraitだけに依存する。アプリ層の各ユースケースは、`CommandRunner`（外部コマンドの実行）・`DiskInfo`・`PrivilegedOps`をコンストラクタで受け取り、テストでは偽実装を渡す
- UI層はディスクを直接触らない。Rustから受け取るのはドメインモデルとエラーの種別（コード）で、表示の文言はすべてUIの辞書で引く。例外は`diskutil`の標準エラー出力と、実行したコマンドの文字列で、これは原文のまま見せる（S01h、S03）

## 2. リポジトリの構成（委任事項の決定）

```
esp_latch/
├─ package.json・bun.lock・svelte.config.js・vite.config.ts・tsconfig.json・eslint.config.js
├─ project.inlang/            Paraglideの設定（locales: ja, en, zh-Hans。baseLocale: ja）
├─ messages/{ja,en,zh-Hans}.json  辞書（jaが原文で正本）
├─ src/
│  ├─ app.html
│  ├─ routes/  +layout.ts（ssr=false, prerender）、+page.svelte（S01）、settings/（S02）、log/（S03）
│  └─ lib/
│     ├─ theme/  tokens.css（specsのコピー）、carbon_theme.css（g10・g100の土台）、fonts.css
│     ├─ i18n/   ロケールの判定・適用、疑似ロケール
│     ├─ api/    commands.ts（invoke）、events.ts（listen）、types.ts（Rustの型と一致させる）
│     ├─ stores/ devices・selection・settings・log・privileged・toasts
│     ├─ format/ bytes・time・percent（Intl）
│     └─ components/  DeviceList・DeviceListItem・DeviceDetail・PartitionBand・EspTile・EspUsageBar・PartitionTable・StatusBar・Toolbar・EjectModal・ToastStack・TitleBar
├─ Cargo.toml                 ワークスペース（src-tauri、crates/*）
├─ src-tauri/                 本体（tauri.conf.json、capabilities/、binaries/、icons/）
│  └─ src/  lib.rs・commands/・domain/・disk_info/・boot/・esp/・monitor/・privileged/・oplog/・settings/・runner/
├─ crates/
│  ├─ esp_latch_protocol/
│  └─ esp_latch_helper/       バイナリ名 esp-latch-helper
├─ scripts/  check.ts（checkコマンド）、i18n_check.ts、pseudo_locale.ts、scan_hardcoded.ts、build_helper.sh（アーキテクチャごとのビルドとlipo）、test_image.sh（開発用のディスクイメージ）
└─ docs/  brand/（brand.md・icon_master.svg・icon_layers/）、images/
```

- 版の確認：Tauri 2（スペック調査時点 2.12.0）、`@carbon/web-components` 2.x（同 2.56.0）、SvelteKit（3が安定版なら、Tauri公式のSvelteKitの手順との差を確かめて選ぶ）、Paraglide JS 2、`objc2-disk-arbitration` 0.3系。いずれもT1-1・T1-4・T1-5で、インストールした版と公式ドキュメントで確かめる
- 最低OS：`bundle.macOS.minimumSystemVersion`は`11.0`を基本とし、依存クレートとTauriの要件がそれより上なら引き上げてCONTRIBUTING.mdとREADMEに書く
- ビルド対象：`universal-apple-darwin`だけ（rustupに`aarch64-apple-darwin`と`x86_64-apple-darwin`を入れる）。Windows版は作らない。署名・公証はしない

## 3. ドメインモデル（Rust。UIの`api/types.ts`と一致させる）

- `Device`：`id`（`disk2`）、`kind`（`internal`・`external`・`image`）、`labels`（表示名の候補。先頭が大きく出る名前）、`extra_label_count`（「ほかN個」）、`vendor`、`image`（`{ path, stored_on }`。`stored_on`は`disk1s1`のようなBSD名）、`connection`（USB・PCI-Express・ディスクイメージなどの種別）、`scheme`（GPT・MBR）、`size`、`partitions`
- `Partition`：`id`（`disk2s1`）、`kind`（`esp`・`apfs`・`hfs`・`other`）、`type_name`（`diskutil`の表示名）、`name`、`offset`、`size`、`part_uuid`（GPTのパーティションUUID）、`mount_point`、`apfs`（APFSパーティションのとき、`ApfsContainer`）
- `ApfsContainer`：`id`（`disk3`）、`volumes`（`id`・`name`・`mount_point`・`used`）
- `EspDetail`（マウント中のESPだけ）：`mount_point`、`loader`（`{ name: opencore|clover, version: Option<String> }`。なければ`None`）、`usage`（`{ total, free, trash }`。読めない値は`None`）
- `DeviceSnapshot`：`devices`、`boot_part_uuid`（OpenCoreの起動パスから取ったUUID。なければ`None`）、`taken_at`
- 「起動に使用」は、`Partition.part_uuid == boot_part_uuid`かつ`kind == esp`のときにUIで印をつける
- ラベル名の選び方（DiskutilAdapterに置く。スペック2章のとおり）：ESPを除く最初のボリューム。APFSは`/`にマウントされるボリューム、なければ最初のマウント中のボリューム。ほかのボリュームの数を`extra_label_count`にする。ラベルがなければベンダー名、ディスクイメージはイメージのファイル名を先頭にする
- サイドバーに出すのは物理ディスクとディスクイメージだけ（`diskutil list -plist`の`WholeDisks`のうち、APFSコンテナの合成ディスクを除く）。合成ディスクは`Partition.apfs`の入れ子としてだけ持つ

## 4. アプリ層のインターフェース（Tauriコマンドとイベント）

コマンド（すべて`Result<T, AppError>`）：

- `list_devices() -> DeviceSnapshot`
- `refresh()`：再取得して`devices-changed`を出す（更新ボタン）
- `inspect_esp(part: String) -> EspDetail`：マウント中のESPの詳細（ブートローダー、容量）
- `mount_esp(part: String) -> EspDetail`
- `unmount_esp(part: String, empty_trash: bool)`
- `open_esp(part: String)`：マウント先を引き直してFinderで開く（UIからパスを受け取らない）
- `eject_device(disk: String)`
- `empty_esp_trash(part: String) -> u64`（解放したバイト数）
- `get_settings() -> Settings`、`update_settings(patch) -> Settings`
- `get_log() -> Vec<LogEntry>`、`clear_log()`
- `open_window(kind: settings|log)`：すでに開いていれば前面に出す

イベント：

- `devices-changed`（`DeviceSnapshot`）
- `log-appended`（`LogEntry`）、`log-cleared`
- `privileged-status`（`none`・`waiting_auth`・`running`）
- `settings-changed`（`Settings`）。すべてのウィンドウが受け取り、言語・外観をその場で反映する
- `busy`（`{ target, op }`の開始と終了。ボタンを処理中にする）

`AppError`の種別（UIは種別を辞書のキーに対応づける）：`auth_cancelled`、`command_failed { command, stderr }`、`not_esp`、`invalid_target`、`busy`、`helper_unavailable`、`not_mounted`、`io`。

- 同じディスクへの操作は同時に1つだけにする（アプリ層のロック。2つ目は`busy`）
- 引数はアプリ層でもBSD名の形式を検証し、補助プロセスでもう一度検証する

## 5. 特権操作

### 起動と認証（HelperGateway）

1. 権限が要る操作を初めて受けたら、`privileged-status: waiting_auth`を出す
2. 本体が一時ディレクトリ（`$TMPDIR`の下、0700）を作り、トークン（ランダム32バイト）をファイル（0600）に書く
3. `osascript -e 'do shell script "<helperのパス> --dir <一時ディレクトリ> --parent-pid <本体のPID> >/dev/null 2>&1 &" with administrator privileges'`で起動する。引数はシェル用にクオートする。`do shell script`はコマンドの終了まで戻らないため、補助プロセスをバックグラウンドで起動して即座に戻らせる
4. osascriptがエラー-128で終わったら`auth_cancelled`（S01g）。それ以外の失敗は`helper_unavailable`
5. 一時ディレクトリにソケットが現れるのを待って（タイムアウトつき）接続し、`privileged-status: running`を出す。以後はアプリを閉じるまで使い回す
6. 本体の終了時に`shutdown`を送る。補助プロセスは、本体のPIDの終了（kqueueの`NOTE_EXIT`）でも自分で終了する

補助プロセスの起動時の検査：一時ディレクトリが実体のディレクトリ（シンボリックリンクでない）で、所有者が一般ユーザー、パーミッションが0700であること。トークンのファイルを読んだら消す。ソケットを作り、所有者をディレクトリの所有者のUIDにし、0600にする。接続ごとに`getpeereid`で相手のUIDがディレクトリの所有者と一致することを確かめる。

### 通信（esp_latch_protocol）

- 改行区切りのJSON。要求：`{ token, id, op }`、応答：`{ id, ok, commands: [{ command, status, stderr }], freed_bytes? , error? }`
- `op`は許可リストだけ：`mount_esp { part }`、`unmount_esp { part }`、`eject_disk { disk }`、`empty_trash { part }`、`shutdown`
- 検証：`part`は`^disk[0-9]+s[0-9]+$`、`disk`は`^disk[0-9]+$`。`mount_esp`・`unmount_esp`・`empty_trash`は、補助プロセスが`diskutil info -plist`を引き直し、パーティションの種別がEFI（GUID `C12A7328-F81F-11D2-BA4B-00A0C93EC93B`、またはMBRの0xEF）であることを確かめる
- 補助プロセスが実行したコマンドは`commands`で返し、アプリ層が操作ログに記録する

### 各操作

- `mount_esp`：`diskutil mount <part>`。失敗したら、`/Volumes/EFI`（使われていれば`EFI 1`、`EFI 2`…）を作り、`mount -t msdos -o -u=<UID>,-g=<GID> /dev/<part> <マウントポイント>`に切り替える（UID・GIDは接続相手のもの。一般ユーザーがFinderで書き込めるようにする）。両方のコマンドを返す
- `unmount_esp`：`diskutil unmount <part>`。フォールバックで作ったマウントポイントは、取り外し後に空なら消す
- `eject_disk`：`diskutil eject <disk>`
- `empty_trash`：BSD名からマウント先を引き直し、`<マウント先>/.Trashes`が実体のディレクトリであることを`lstat`で確かめ、シンボリックリンクをたどらずに中身を消す。解放量は、消す前後の`statfs`の空き容量の差。ログには`rm -rf <マウント先>/.Trashes`の形で記録する（モックの表記）。付随ファイル（`._*`、`.fseventsd`）は消さない

## 6. 読み取り系の詳細

- **DiskutilAdapter**：`diskutil list -plist`、各ディスクの`diskutil info -plist`、`diskutil apfs list -plist`、`hdiutil info -plist`を実行して組み立てる。plistの解析は`plist`クレート。単体テストは実機の出力から作ったフィクスチャで行う（ボリューム名・UUID・シリアルは架空の値に置き換える）
- **操作ログの粒度**：一覧の取得1回につき、`diskutil list -plist`の1行だけを記録する。読み取りの途中でコマンドが失敗したら、そのコマンドを`fail`で別の行に残す
- **NvramBootInfo**：`nvram 4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:boot-path`の出力から`GPT,<UUID>`を取り出す。値がなければ`None`（Cloverでの起動や実機のMacでは印を出さない）
- **EspInspector**
  - ブートローダー：`EFI/OC/OpenCore.efi`があればOpenCore、`EFI/CLOVER/CLOVERX64.efi`があればClover。版は、バイナリに埋め込まれた版の文字列から読む（OpenCoreは`REL-107-…`のようなビルドの文字列から`1.0.7`を得る、CloverはRevisionの文字列。正確な形式はT5-4で実物のバイナリを調べて確定し、バイト列のフィクスチャで単体テストする）。読めなければ版を`None`にする
  - 容量：マウント先の`statfs`。ゴミ箱の量は`<マウント先>/.Trashes`以下の合計（読めないものがあれば`trash`を`None`にし、UIは「不明」を出す）。空きが15%未満なら、UIで強調する（S01k）
- **DeviceMonitor**：専用スレッドでCFRunLoopを回し、DiskArbitrationのappeared・disappeared・description changedのコールバックを受ける。最後のイベントから500msで再取得を1回だけ行い（間引き）、`devices-changed`を出す。設定の自動更新がオフのときは、コールバックの登録を外す
- **OperationLog**：`LogEntry { at（UNIX時刻ms）, command, result: ok|fail|cancelled }`のリスト。上限1000件（古いものから捨てる）。ファイルには保存しない。ステータスバーの「直前のコマンド」は最後の記録
- **SettingsStore**：アプリの設定ディレクトリの`settings.json`。Tauriのstoreプラグインを使うか、serdeで直接読み書きするかはT2-6で決める

## 7. 永続化するデータ（設定）

構造：

```
Settings {
  language: "system" | "ja" | "en" | "zh-Hans",   既定 "system"
  theme: "system" | "light" | "dark",             既定 "system"
  default_filter: "all" | "esp" | "physical",     既定 "all"
  open_finder_after_mount: bool,                  既定 true
  auto_refresh: bool,                             既定 true
  confirm_eject: bool,                            既定 true
  auto_empty_trash_on_unmount: bool,              既定 false
  show_command: bool,                             既定 true
}
```

動き（架空の利用者Aさんの1日）：

1. Aさんが初めて起動する → 設定ファイルはまだない。既定値をメモリに持ち、ファイルは作らない。操作ログは空（S03a）
2. 一覧が出る（S01e → S01）→ 設定は変わらない。操作ログに`diskutil list -plist`が1行増える
3. USBメモリのESPをマウントする → 初回なので認証（S01f）。設定は変わらない。操作ログに`osascript …`・`diskutil mount disk4s1`・`open /Volumes/EFI`が増える（`open`は`open_finder_after_mount`がtrueのときだけ）
4. 設定（S02）で言語を「English」にする → `settings.json`が作られ、`language: "en"`だけが既定と異なる値で保存される。開いているすべてのウィンドウが英語に切り替わる
5. 設定で「取り外すときにゴミ箱を自動で空にする」をオンにする → `auto_empty_trash_on_unmount: true`が保存される
6. ESPを取り外す → 設定は変わらない。操作ログに`rm -rf /Volumes/EFI/.Trashes`と`diskutil unmount disk4s1`が増える
7. アプリを終了する → 操作ログは消える。補助プロセスも終了する。設定ファイルは残る
8. 翌日起動する → `settings.json`から英語・ゴミ箱の自動削除オンを読む。`default_filter`に従って絞り込みを選んだ状態で始まる

画面の項目と設定の対応（S02）：言語→`language`、外観→`theme`、起動時の絞り込み→`default_filter`、マウントしたらFinderで開く→`open_finder_after_mount`、検知して一覧を更新→`auto_refresh`、取り出す前に確認→`confirm_eject`、取り外し時にゴミ箱を自動で空にする→`auto_empty_trash_on_unmount`、コマンドをステータスバーに表示→`show_command`。バージョンとリポジトリは表示だけ（保存しない）。

画面の項目と、設定以外のデータの対応：S01のサイドバー・詳細・帯・表→`DeviceSnapshot`（保存しない）、S01a・S01kのESPのタイル→`EspDetail`（保存しない）、S01のステータスバー→`privileged-status`と`OperationLog`の最後の記録、S03→`OperationLog`（保存しない）。

削除・修正・二重操作：

- 設定の変更は1項目ずつ即時に保存する（保存ボタンはない）。同じ値への変更は書き込まない
- 2つのウィンドウから同時に変更しても、本体の`SettingsStore`が順に適用し、最後の値が`settings-changed`で全ウィンドウに配られる
- 設定ファイルが壊れている・知らない値がある → その項目だけ既定値にして起動し、次の保存で正しい内容に上書きする
- 将来の版で項目が増えた → ない項目は既定値で補う
- 操作ログの消去 → メモリのリストを空にし、`log-cleared`を出す。設定には影響しない

## 8. UI層

- **ウィンドウ**：`main`（1100×720。最小サイズはT3-2でサイドバーと詳細が崩れない値に決める）、`settings`（S02）、`log`（S03）。設定と操作ログは、ツールバーのボタンとアプリメニュー（「設定…」⌘,）から開き、2つ目は作らず前面に出す
- **テーマ**：`carbon_theme.css`でCarbonのg10（ライト）とg100（ダーク）の土台を`[data-theme]`に当て、そのあとに`tokens.css`を読み込んで上書きする。`theme`が`system`のときは`prefers-color-scheme`を見て`data-theme`を付け替える。`--esp-radius-*`は、Carbonの部品には最小限だけ当てる（スペック6章）
- **書体**：IBM Plex Sans・Sans JP・Monoを同梱する（Google FontsなどのCDNを使わない。ネットワークに接続しない要件のため）。ロケールごとの切り替えは`tokens.css`の`:lang()`に任せ、`<html lang>`を現在のロケールにする。識別子・容量・マウント先・コマンドは`--esp-font-mono`
- **i18n**
  - 3つのモックの辞書（118キー）を`messages/{ja,en,zh-Hans}.json`へ移し、複数形の形をそろえる（モックではjaの`device_list.count`だけ単純な文字列）。移した時点で`messages/ja.json`が文言の正本
  - ロケールの決め方：設定の`language` → `system`なら、OSの優先言語（`ja*`→ja、`zh-Hans`・`zh-CN`・`zh-SG`→zh-Hans、`en*`→en）→ どれでもなければja
  - 日付・時刻・数値・容量は`Intl`にロケールを渡して整形する。文字列の連結で文を組み立てない
  - Carbonの部品が持つ文言（閉じるボタンのラベルなど）は、辞書から属性で渡す
  - アプリメニューの文言も辞書から作る（フロントエンドからTauriのメニューAPIで組み立て、ロケールが変わったら作り直す）
  - 実装で新しいキーを足したら、en・zh-Hansの訳案も同時に書く（checkコマンドでキーの過不足が検出される）。訳はユーザーが各Phaseの言語の確認で見る
  - 疑似ロケール：jaの文言を3〜4割伸ばし`［］`で囲んだ辞書をスクリプトで作る。開発時だけ有効にし（環境変数で切り替え）、公開版の言語には含めない
- **部品**：スペック6章の「そのまま使う／組み合わせて作る／独自に作る」に従う。独自の部品（帯グラフ、容量バー、ステータスバー、タイトルバー周り）の色・余白・書体は、Carbonのトークン（`--cds-*`）と`--esp-*`だけを参照する
- **アクセシビリティ**：状態は色だけでなく文字でも示す。帯グラフの各区画に、識別子・名前・容量のツールチップ（キーボードのフォーカスでも出す）

## 9. 自動検査（checkコマンド）

`bun run check`で次を順に実行し、失敗した段階の出力だけを表示する。CIも同じコマンドを呼ぶ。

1. `svelte-check`（型）
2. `eslint`
3. `vitest run`（format・stores・i18nの関数）
4. `cargo fmt --check`、`cargo clippy --workspace -- -D warnings`、`cargo test --workspace`
5. i18n：ロケール間のキーの過不足、プレースホルダの一致、複数形の形、未使用キー、`.svelte`・`.ts`内の文字列の直書き（許可リストつき）
6. `src/lib/theme/tokens.css`と`roadmap/specs/`の最新の`esp_latch_tokens_v*.css`の一致、`docs/brand/`のコピーと`specs/`の一致
7. 色・サイズの直書きの検出（`src/`の`.svelte`・`.css`で、`tokens.css`・`carbon_theme.css`以外に16進の色や`px`の直書きがないか。許可リストつき）

## 10. 開発用のディスクイメージ

`scripts/test_image.sh`：内蔵ディスクに触れずに試せるよう、ESPを含むGPTのスパースイメージを作って接続・切断する。オプションで、ESPのファイルシステムを壊したイメージ（マウントの失敗 S01hの確認用）と、MBR・exFATのイメージ（S01cの確認用）を作る。必要な権限と手順はT2-1で確かめ、CONTRIBUTING.mdに書く。

## 11. アイコン

- `tauri icon`でマスターSVGから`.icns`と各サイズのPNGを作り、リポジトリに含める（CIでは作らない）
- 前景の4グループ（`01_body`・`02_band`・`03_esp`・`04_latch`）を`docs/brand/icon_layers/{id}.svg`に書き出す（`00_bg`は書き出さない）
- ユーザーがIcon Composerで作った`AppIcon.icon`を、tauri-bundler 2.9系以降の`.icon`の組み込みで同梱し、`.icns`をフォールバックに残す。対応状況と既知の不具合は、T7-4の時点の公式ドキュメントで確かめる

## 12. 配布

- GitHub Actions：`v*`タグのpushで起動し、checkコマンド → 補助プロセスのユニバーサルビルド（`build_helper.sh`）→ tauri-actionで`universal-apple-darwin`の`.dmg`を作り、ドラフトのリリースを作る
- 補助プロセスは`src-tauri/binaries/esp-latch-helper-<ターゲットトリプル>`に置く（`externalBin`）。Universalビルド用の命名は、T5-2の時点のTauriの公式ドキュメントで確かめる
