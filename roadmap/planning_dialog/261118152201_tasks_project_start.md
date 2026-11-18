# esp_latch tasks

設計は`261118152201_imple_project_start.md`、正本は`roadmap/specs/`の最新版。実行順は記載順。

## Phase 1: 基盤（雛形・テーマ・i18n・checkコマンド）
Phaseの推奨: Sonnet・high以上
引き継ぎ: Phase 1完了（W1-1〜W1-3通過）。次はT2-1（Phase 2はOpus・high以上）。RustはHomebrewの`rustup`（keg-only）で、`PATH`の先頭に`/opt/homebrew/opt/rustup/bin`が要る（ユーザーは`~/.zshrc`に追記済み）。Carbonの部品はShadow DOM内で`IBM Plex Sans`を直書きしており和文がsystem-uiになる点は、Phase 3で扱う
- [x] T1-1 【エージェント】プロジェクトの雛形を作る：SvelteKit（`adapter-static`、`ssr = false`のSPA）＋Tauri 2＋Bun。Cargoワークスペース（`src-tauri`、`crates/esp_latch_protocol`、`crates/esp_latch_helper`の空の骨格）。識別子`com.rinontech.esp-latch`、表示名ESP Latch、メインウィンドウ1100×720、`minimumSystemVersion`。rustupに`aarch64-apple-darwin`と`x86_64-apple-darwin`を入れ、Universalのビルドが通るところまで
  - SvelteKit 3が安定版なら、Tauri公式のSvelteKitの手順との差を確かめて版を選ぶ（imple 2章）
  - 実施: Opus・?
  - SvelteKit 3.3（安定版）を採用。Tauri公式手順（2.20時点）のadapter-static・`fallback: 'index.html'`・`ssr = false`はそのまま使え、差は`svelte.config.js`の廃止（`vite.config.ts`の`sveltekit()`へ）とTypeScript 6・Vite 8の要件だけ。Homebrewのrustをrustupに置き換えた
- [x] T1-2 【エージェント】`CONTRIBUTING.md`の骨格（受け付け方針、開発環境、セットアップ、起動のコマンド、リポジトリ構成と`roadmap/`・`docs/brand/`の役割）
  - 推奨: Sonnet・medium以上
  - 依存: T1-1
  - 実施: Opus・?
- [x] T1-3 【ユーザー】claude.aiでトークンのコントラスト（ライトの`--esp-state-mounted`、ダークの`--cds-button-primary`・`-hover`）を調整し、改訂版プランアセットをリポジトリルートに置く
  - 実施: ユーザー（claude.ai）。プランアセット v2として取り込み済み ← development/261118152033_change_token_contrast.md
- [x] T1-4 【エージェント】Carbon Design Systemの導入とテーマの設定：`@carbon/web-components` 2.xを導入し、g10・g100の土台を`[data-theme]`に当て、`specs/`の`tokens.css`を`src/lib/theme/tokens.css`へコピーして上書きさせる。外観（システム・ライト・ダーク）の切り替えの仕組み。IBM Plex Sans・Sans JP・Monoを同梱する（CDNを使わない）。`brand.md`とマスターSVGを`docs/brand/`へコピーする
  - 依存: T1-1, T1-3
  - 実施: Opus・?
  - `@carbon/web-components` 2.67.0。土台は`carbon_theme.scss`（`@carbon/styles`の`_zone.scss`にならい`[data-theme]`へ出力。値をファイルに持たないためSCSSにした）。Carbonの部品はShadow DOM内で`IBM Plex Sans`を直書きしているため、ja・zh-Hansの和文はsystem-uiで出る。Phase 3で部品に当てるときに扱う
- [x] T1-5 【エージェント】i18nの基盤：Paraglide JS 2を導入し、3つのモックの辞書を`messages/{ja,en,zh-Hans}.json`へ移す（複数形の形をそろえる）。ロケールの判定・保持・フォールバック、`<html lang>`、ロケールごとの書体、Carbonの部品への文言の受け渡し、`Intl`による整形の関数、疑似ロケール（開発時だけ）
  - 推奨: Opus・high以上
  - 依存: T1-4
  - 実施: Opus・?
  - Paraglide JS 2.29.3（3.xは11月3日に出たばかりのMAJORのため、スペックどおり2系）。キーの衝突を避けるため`settings.language`・`settings.theme`を`*.label`に改めた（キー設計は委任事項）。OSの優先言語はWebViewでなくRust（sys-locale）から読む
- [x] T1-6 【エージェント】checkコマンド`bun run check`：型検査・lint・テスト（TypeScriptとRust）、i18nの自動検査、トークンと`docs/brand/`のコピーの一致、色・サイズの直書きの検出を1つにまとめ、失敗した段階だけを表示する（imple 9章）。`CONTRIBUTING.md`に追記する
  - 推奨: Opus・high以上
  - 依存: T1-5
  - 実施: Opus・?
  - 未使用キーは、画面がまだない分を`scripts/i18n_pending.json`に置いて許す（使い始めたら外す）。検査で見つかったenの`device.mounted_count`の`one`の不足を補った
- [x] T1-7 【エージェント】アプリの骨格：メイン・設定・操作ログの3つのウィンドウ（`open_window`。2つ目は作らず前面に出す）、アプリメニュー（「設定…」⌘,を含む。文言は辞書から作り、ロケールの変更で作り直す）、各ウィンドウの空の枠
  - 依存: T1-6
  - 画面の中身はPhase 3・4で実装する
  - 実施: Opus・?
  - メインウィンドウは閉じても隠すだけにした（アプリメニューを組み立てる役を持つため。Dockのクリックとウインドウメニューで再表示）。メニュー用のキー（`menu.*`）を辞書に追加。Carbonの部品への誤検知のa11y警告2種を`cds-*`に限って外した

## Phase 2: ディスク情報とアプリ層（読み取りと設定）
Phaseの推奨: Opus・high以上
引き継ぎ: T2-1〜T2-6完了。次はW2-1。W2-2・W2-3は`scripts/test_image.sh attach all`でイメージを接続してから`cargo run -p esp_latch --example dump_devices`（`-- --watch`で検知の確認）。一覧の取得は実機で約0.8秒
- [x] T2-1 【エージェント】開発用のディスクイメージのスクリプト`scripts/test_image.sh`：ESPを含むGPTのスパースイメージ、ESPのファイルシステムを壊したイメージ、MBR・exFATのイメージを作り、接続・切断する。必要な権限を確かめ、`CONTRIBUTING.md`に使い方を書く
  - 推奨: Sonnet・high以上
  - 依存: T1-7
  - 実施: Opus・high
  - 管理者権限は不要（接続したイメージのデバイスは接続したユーザーの所有）。512MBではdiskutilがESPを自動で作らないため、ESPの種別GUIDで区画を作り`newfs_msdos`でFAT32にした。`hdiutil`のcreate・attachはmacOS 26以降で非推奨だが、最低OSで動くようhdiutilを使う
- [x] T2-2 【エージェント】ドメインモデル（Device・Partition・ApfsContainer・Volume・DeviceSnapshot）と`CommandRunner`、`DiskutilAdapter`（`diskutil list`・`info`・`apfs list`、`hdiutil info`のplistの変換、合成ディスクの除外、ラベル名の選び方、ディスクイメージのパスと保存先）。実機の出力から作ったフィクスチャ（名前・UUID・シリアルは架空の値に置き換える）で単体テスト
  - 依存: T2-1
  - 実施: Opus・high
  - `diskutil apfs list`は使わない（コンテナのボリューム・マウント先・使用量は`diskutil list`にある）。`diskutil info`は1回0.1〜0.2秒かかるため8並列で実行。`Partition`にモックの「形式」列のための`filesystem`を足し、`name`・`vendor`はない場合に`None`とした。ラベルはOSが内部で使うAPFSのボリューム（`OSInternal`）を除き、`/`はスナップショットのマウント先からも判定する。保存先は`statfs`で引く。フィクスチャは`src-tauri/fixtures/diskutil/`
- [x] T2-3 【エージェント】`NvramBootInfo`：OpenCoreの`boot-path`からGPTのパーティションUUIDを取り出す。値がない場合を含めて単体テスト
  - 推奨: Sonnet・high以上
  - 依存: T2-2
  - 実施: Opus・high
  - 手元のMacには値がないため、テストの値はOpenCoreの`boot-path`の形式（`HD(1,GPT,<UUID>,…)`）から作った。値がない・GPTでない・形が崩れている・nvramが失敗した場合は`None`
- [x] T2-4 【エージェント】アプリ層：`list_devices`・`refresh`、`devices-changed`、`OperationLog`（上限、`log-appended`・`log-cleared`、`get_log`・`clear_log`、読み取りを1行にまとめる粒度）、`AppError`の種別、同じディスクへの操作のロック。開発用の`cargo run --example dump_devices`
  - 依存: T2-3
  - 実施: Opus・high
  - `AppCore`はTauriに依存せず`EventSink`へイベントを出す（テストは記録用の実装）。`list_devices`は取得済みの一覧を返し、`refresh`だけが取得し直して`devices-changed`を出す。`boot-path`は最初の取得で1回だけ読む。ロックは`begin_operation`で取り、`busy`の開始・終了を出す（Phase 5の操作で使う）。BSD名の検証は`esp_latch_protocol::bsd`に置いた。UIの型と窓口（`api/types.ts`・`commands.ts`・`events.ts`）も追加。実機で一覧の取得は約0.8秒
- [x] T2-5 【エージェント】`DeviceMonitor`：DiskArbitration（`objc2-disk-arbitration` 0.3系）のコールバックを専用スレッドで受け、500msの間引きで再取得して`devices-changed`を出す。開始・停止のAPI
  - 依存: T2-4
  - 実施: Opus・high
  - `objc2-disk-arbitration` 0.3.2。CFRunLoopは`Send`でないため、停止はフラグと0.25秒ごとの`run_in_mode`で確かめる。登録直後に届く既存のディスクの通知は0.3秒読み捨てる（起動直後の一覧の取得と重ならないように）。`dump_devices -- --watch`で、イメージの切断・接続・ESPの取り外しがそれぞれ1回の再取得になることを実機で確認。起動時に開始するよう配線（設定との接続はT2-6）
- [x] T2-6 【エージェント】`SettingsStore`：`get_settings`・`update_settings`・`settings-changed`、既定値、壊れたファイル・知らない値・ない項目の扱い（imple 7章）。`auto_refresh`を`DeviceMonitor`の開始・停止につなぐ。単体テスト
  - 依存: T2-5
  - 実施: Opus・high
  - storeプラグインは使わずserdeで直接読み書き（項目ごとに既定値へ戻す扱いを自前で持つため）。保存は一時ファイルからの置き換え。`settings-changed`はロックを持ったまま出し、順を適用の順とそろえる。`auto_refresh`をオンにして監視を新しく始めたときは1回再取得する。`tauri dev`で起動し、パニックがなく、起動だけでは設定ファイルを作らないことを確認

## Phase 3: メインウィンドウ（一覧と詳細）
Phaseの推奨: Sonnet・high以上
- [ ] T3-1 【エージェント】UIの`api/`（型つきのinvokeとlisten、Rustの型と一致する`types.ts`）と`stores/`（devices・selection・settings・log・privileged・toasts）、`AppError`の種別から辞書のキーへの対応
  - 依存: T2-6
- [ ] T3-2 【エージェント】メインウィンドウの枠：タイトルバー周り、ツールバー（更新・設定・操作ログ）、ステータスバー（準備完了・読み込み中・直前のコマンド。`show_command`に従う）、ウィンドウの最小サイズ
  - 依存: T3-1
  - 画面: S01, S01e
- [ ] T3-3 【エージェント】サイドバー：デバイスの一覧項目（ラベル名・ベンダー名・ほかN個・種別・ESPの数・マウント中の数・小さな帯）、絞り込み（全て・ESPあり・物理、件数、起動時の絞り込みの設定）、空の状態、スケルトン、長い名前の省略とホバーでの全文
  - 依存: T3-2
  - 画面: S01, S01d, S01e, S01j
- [ ] T3-4 【エージェント】詳細の見出しと情報（識別子・接続・方式・容量、ディスクイメージのパスと保存先）、パーティションの帯グラフ（最小幅、凡例、ツールチップ）、ESPのないデバイスの表示
  - 依存: T3-3
  - 画面: S01, S01b, S01c, S01j
- [ ] T3-5 【エージェント】ESPのタイル（表示）：未マウント・マウント中の状態の文字、「起動に使用」の印、マウント先。マウントのボタンは表示し、操作はT5-6で接続する（それまでは無効）
  - 依存: T3-4
  - 画面: S01
- [ ] T3-6 【エージェント】その他のパーティションの表：データテーブルの展開行で、APFSパーティションの下にコンテナとボリュームを入れ子にする
  - 依存: T3-4
  - 画面: S01, S01b

## Phase 4: 設定と操作ログの画面
Phaseの推奨: Sonnet・high以上
- [ ] T4-1 【エージェント】設定ウィンドウ：8つの設定項目とバージョン・リポジトリの表示。変更は即時に保存し、言語・外観・起動時の絞り込み・自動更新・コマンドの表示をその場で反映する（開いているすべてのウィンドウ）。リポジトリのリンクは既定のブラウザで開く
  - 依存: T3-6
  - 画面: S02
- [ ] T4-2 【エージェント】操作ログのウィンドウ：時刻・コマンド・結果の表（追記をその場で反映）、コピー、消去、空の状態
  - 依存: T4-1
  - 画面: S03, S03a

## Phase 5: 特権操作とESPのマウント・取り外し
Phaseの推奨: Opus・high以上
- [ ] T5-1 【エージェント】`esp_latch_protocol`（要求・応答の型、BSD名の検証）と補助プロセス`esp-latch-helper`：一時ディレクトリとトークンの検査、ソケット（所有者とパーミッション、`getpeereid`）、本体のPIDの監視、許可リストの操作（`mount_esp`のフォールバックを含む、`unmount_esp`、`eject_disk`、`empty_trash`、`shutdown`）とESPであることの確認（imple 5章）。不正な入力を含めた単体テスト
  - 依存: T2-6
- [ ] T5-2 【エージェント】`scripts/build_helper.sh`：補助プロセスをアーキテクチャごとにビルドしてlipoでユニバーサルにし、`externalBin`の命名で`src-tauri/binaries/`に置く。Tauriのビルドの前に呼ぶ。`CONTRIBUTING.md`を更新する
  - 推奨: Sonnet・high以上
  - 依存: T5-1
- [ ] T5-3 【エージェント】`HelperGateway`（osascriptでの起動、エラー-128の判定、ソケットの待ち受けと接続、`privileged-status`、終了時の`shutdown`）と`FakePrivilegedOps`
  - 依存: T5-2
- [ ] T5-4 【エージェント】`EspInspector`：ブートローダーと版の検出（実物のバイナリで版の文字列の形式を確かめ、バイト列のフィクスチャで単体テスト）、空き容量と`.Trashes`の量
  - 推奨: Sonnet・high以上
  - 依存: T5-3
- [ ] T5-5 【エージェント】ユースケース：`mount_esp`（設定に従ってFinderで開く）・`unmount_esp`・`open_esp`・`inspect_esp`、補助プロセスが返したコマンドの記録、`busy`。偽実装で単体テスト
  - 依存: T5-4
- [ ] T5-6 【エージェント】UI：ESPのタイルの操作（マウント・取り外し・Finderで開く、処理中）、ブートローダーと容量の表示、ステータスバー（管理者権限で実行中・認証待ち）、通知（マウントの成功、認証のキャンセルと再試行、マウントの失敗とコマンド・ログを見る）
  - 推奨: Sonnet・high以上
  - 依存: T5-5
  - 画面: S01a, S01f, S01g, S01h

## Phase 6: 取り出しとESPのゴミ箱
Phaseの推奨: Sonnet・high以上
- [ ] T6-1 【エージェント】ユースケース：`eject_device`、`empty_esp_trash`（解放量）、ゴミ箱を空にして取り外す、取り外し時の自動削除（`auto_empty_trash_on_unmount`）。偽実装で単体テスト
  - 依存: T5-6
- [ ] T6-2 【エージェント】UI：デバイスの取り出しと確認のモーダル（`confirm_eject`に従う）、ゴミ箱の警告と量、空きが15%未満の強調、「ゴミ箱を空にする」「ゴミ箱を空にして取り外す」、解放量の通知
  - 依存: T6-1
  - 画面: S01i, S01k, S01l

## Phase 7: アイコン
Phaseの推奨: Sonnet・medium以上
- [ ] T7-1 【エージェント】アイコンの展開：`docs/brand/`のマスターSVGから`tauri icon`で`.icns`と各サイズのPNGを作り、`src-tauri/icons/`に置く。コマンドと入力形式はインストール済みの版の公式ドキュメントで確かめる
  - 依存: T6-2
- [ ] T7-2 【エージェント】マスターSVGの前景グループ（`01_body`・`02_band`・`03_esp`・`04_latch`）を、`docs/brand/icon_layers/{id}.svg`として書き出す（`00_bg`は書き出さない）
  - 依存: T7-1
- [ ] T7-3 【ユーザー】レイヤーのSVGと`brand.md`の「Apple環境向けアイコン」の方針から、Icon Composerで`AppIcon.icon`を作り、`src-tauri/icons/AppIcon.icon`に置く
  - 依存: T7-2
- [ ] T7-4 【エージェント】`.icon`をビルドに組み込み、`.icns`をフォールバックに残す。tauri-bundlerの対応状況と既知の不具合を、その時点の公式ドキュメントで確かめる
  - 推奨: Sonnet・high以上
  - 依存: T7-3

## Phase 8: CIとREADME
Phaseの推奨: Sonnet・high以上
- [ ] T8-1 【エージェント】GitHub Actionsのワークフロー：`v*`タグのpushで起動し、checkコマンド → `build_helper.sh` → tauri-actionで`universal-apple-darwin`の`.dmg`を作り、ドラフトのリリースを作る
  - 依存: T7-4
- [ ] T8-2 【エージェント】`CONTRIBUTING.md`の仕上げ（ビルド、アイコンの生成、CIとリリースの手順、デザインシステムとテーマの設定場所）
  - 推奨: Sonnet・medium以上
  - 依存: T8-1
- [ ] T8-3 【ユーザー】メインウィンドウのスクリーンショット（ライト・ダーク）を日本語・英語・中国語で撮り、エージェントが示すファイル名で`docs/images/`に置く
  - 依存: T7-4
- [ ] T8-4 【エージェント】`README.md`（日本語）、`README.en.md`、`README.zh-Hans.md`と、GitHubのAbout欄の文案（英語の説明文・トピック）。注意事項（起動不能のリスク、署名・公証がない場合の警告と開き方、管理者権限を使う理由と範囲、ESP上の「ゴミ箱に入れる」で容量が戻らないこと）を含める
  - 依存: T8-3
- [ ] T8-5 【エージェント】モックの辞書から追加・変更したキーの一覧と、en・zh-Hansの訳を、ユーザーが確認できる形で出す
  - 依存: T8-4

## Phase 9: 初回リリース
Phaseの推奨: Sonnet・high以上
- [ ] T9-1 【エージェント】リリース準備 0.1.0：`CHANGELOG.md`、バージョン表記をそろえる、ローカルのUniversalビルド、checkコマンド、アイコン一式と`.icon`のレイヤーが最新のマスターSVGと一致すること、READMEが実物と一致すること、`timeline.md`へのリリース準備の行。コミットメッセージとタグ作成・pushのコマンドを提示して停止する
  - 依存: T8-5
- [ ] T9-2 【ユーザー】コミット、タグの作成とpushを行い、CIがドラフトのリリースを作ったことを確かめる
  - 依存: T9-1
