# ESP Latchの開発

不具合の報告はIssueで受け付けます。プルリクエストは、事前にIssueで相談してください。

## 開発環境

- macOS 11以降（Apple silicon・Intelのどちらでも可）と、Xcode Command Line Tools
- [Bun](https://bun.sh/) 1.4以降（パッケージ管理とスクリプトの実行。Node.jsは使いません）
- [rustup](https://rustup.rs/)で入れたRustの安定版と、2つのターゲット（Universalビルド用）

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
rustup component add clippy rustfmt
```

HomebrewでRustを入れる場合は、`rust`ではなく`rustup`を入れます（`rust`は他のターゲットを追加できません）。`rustup`はkeg-onlyのため、`PATH`の先頭に`$(brew --prefix rustup)/bin`を加えてください。

## セットアップ

```sh
bun install
```

## 起動

```sh
bun run tauri dev
```

最初に補助プロセスをこのMacのアーキテクチャ向けにビルドし（`scripts/build_helper.sh --host`）、Viteの開発サーバー（`http://localhost:1420`）が起動したあとに、アプリのウィンドウが開きます。

設定は`~/Library/Application Support/com.rinontech.esp-latch/settings.json`に保存されます。最初に設定を変えるまでファイルは作られません。消すと既定値に戻ります。操作ログはメモリにだけあり、アプリを終了すると消えます。

## 開発用のディスクイメージ

内蔵ディスクや手持ちのUSBメモリに触れずに試せるよう、`scripts/test_image.sh`でディスクイメージを作り、接続・切断できます。

```sh
scripts/test_image.sh create all     # 3種類のイメージを作る（各512MBのスパースイメージ。実際の使用量はわずか）
scripts/test_image.sh attach gpt     # 接続する（gpt・broken・mbr・all）
scripts/test_image.sh status         # 作ったイメージと、接続先のディスク（/dev/diskN）
scripts/test_image.sh detach all     # 切断する（ESPがマウントされたままでも取り出す）
scripts/test_image.sh clean all      # 切断して、イメージファイルを消す
```

| 種別 | 構成 | 確かめられること |
| --- | --- | --- |
| `gpt` | GPT。ESP（FAT32・200MB、ラベル`EFI`）と、APFSコンテナ（ボリューム「ESP Latch Test」） | ESPの一覧・マウント・取り外し・取り出し、ディスクイメージの表示 |
| `broken` | `gpt`と同じ構成で、ESPのファイルシステムの先頭を0で埋めたもの（ボリューム「ESP Latch Broken」） | マウントの失敗の通知 |
| `mbr` | MBR。exFATのパーティション1つ（ラベル`ESPLATCHMBR`）。ESPはない | MBRのディスクの表示 |

- 管理者権限は要りません。自分で接続したディスクイメージのデバイス（`/dev/diskN`）は接続したユーザーの所有になるため、パーティションの作成・フォーマットも一般ユーザーで行えます。アプリからの操作は、実際のディスクと同じく補助プロセスを経由します
- イメージは`target/test_images/`にできます（`target/`はリポジトリに含めません）。置き場所は環境変数`ESP_LATCH_TEST_IMAGE_DIR`で変えられます
- パーティションの作成・フォーマット・破壊は、スクリプトが接続したディスクイメージのデバイスだけに行い、ディスクイメージでないことが分かったら中止します
- macOS 26以降では、`hdiutil`の`create`・`attach`が非推奨になり、`diskutil image`への移行を促す警告が出ます。開発環境の最低OS（macOS 11）でも動くよう`hdiutil`を使い、警告は表示しません

### デバイスの一覧をJSONで見る

UIを起動せずに、アプリ層が組み立てたデバイスの一覧（UIが`list_devices`で受け取るJSON）を確かめられます。操作ログとイベントは標準エラー出力に出ます。

```sh
cargo run -p esp_latch --example dump_devices
cargo run -p esp_latch --example dump_devices -- --watch   # 続けてディスクの変化を検知し、再取得のたびに要約を出す
```

単体テストのフィクスチャ（`src-tauri/fixtures/diskutil/`）は、`diskutil`・`hdiutil`の実際の出力から、UUID・ユーザー名・スナップショット名を架空の値に置き換えて作っています。

## 確認とテスト（checkコマンド）

```sh
bun run check
```

次の8段階を順に実行し、失敗した段階の出力だけを表示します。プルリクエストの前と、CIで実行します。

1. 準備：辞書からメッセージ関数を生成し（`scripts/paraglide_compile.ts`）、SvelteKitの型を生成する
2. 型検査：`svelte-check`（警告も失敗とする）
3. lint：`eslint`（警告も失敗とする）
4. テスト：`vitest run`（`src/`と`scripts/`の`*.spec.ts`）
5. Rust：補助プロセスのビルド（`scripts/build_helper.sh --host`。本体のビルドに要る）、`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`
6. i18n（`scripts/i18n_check.ts`）：ロケール間のキーの過不足、プレースホルダの一致、複数形の区分、未使用キー、`.svelte`・`.ts`の文字列の直書き
   - 画面がまだなく使っていないキーは`scripts/i18n_pending.json`に置きます。画面で使い始めたら、そのキーをここから外します（外し忘れも検出します）
   - やむを得ない直書きは、`scripts/i18n_allowlist.json`にファイル・文字列・理由を書いて許可します
7. コピーの一致（`scripts/check_copies.ts`）：`src/lib/theme/tokens.css`と`docs/brand/`のファイルが、`roadmap/specs/`の最新版と同じ内容か
8. 色・サイズの直書き（`scripts/scan_hardcoded.ts`）：`src/`の`.svelte`・`.css`・`.scss`で、`tokens.css`と`carbon_theme.scss`以外に色や`px`を書いていないか。例外は`scripts/hardcoded_allowlist.json`に理由とともに書きます

個別に実行するときは、`bun run test`（vitest）、`bun run lint`（eslint）、`bun scripts/i18n_check.ts`などを使います。

## ビルド

```sh
bun run tauri build --target universal-apple-darwin
```

Apple silicon・Intelの両方で動く`ESP Latch.app`と`.dmg`が`target/universal-apple-darwin/release/bundle/`にできます。`.app`だけでよい場合は`--bundles app`を付けます。コード署名・公証は行いません。

### 補助プロセス（サイドカー）

root権限で動く補助プロセス`esp-latch-helper`は、本体とは別のバイナリで、`.app`の`Contents/MacOS/`に同梱します（`tauri.conf.json`の`bundle.externalBin`）。`tauri build`・`tauri dev`の前に、`beforeBuildCommand`・`beforeDevCommand`から`scripts/build_helper.sh`が自動で呼ばれます。

```sh
scripts/build_helper.sh          # aarch64・x86_64をビルドし、lipoでユニバーサルにする
scripts/build_helper.sh --host   # このMacのアーキテクチャだけ（開発・checkコマンド用）
```

- できるファイルは`src-tauri/binaries/esp-latch-helper-<ターゲットトリプル>`です（`aarch64-apple-darwin`・`x86_64-apple-darwin`・`universal-apple-darwin`）。Tauriは、本体をビルドするアーキテクチャのトリプルの付いたファイルを要求し、Universalのバンドルにはユニバーサルのファイルを同梱します。ファイルがないと、本体の`cargo build`・`cargo clippy`も失敗します。`src-tauri/binaries/`はリポジトリに含めません
- 最低OSは本体にそろえて`MACOSX_DEPLOYMENT_TARGET=11.0`でビルドします
- 同梱されたものが両方のアーキテクチャを含むかは、`lipo -info "target/universal-apple-darwin/release/bundle/macos/ESP Latch.app/Contents/MacOS/esp-latch-helper"`で確かめられます

## リポジトリの構成

```
esp_latch/
├─ package.json・vite.config.ts・tsconfig.json・eslint.config.js
├─ src/                    UI（Svelte 5＋SvelteKit 3のSPA）
│  ├─ app.html
│  ├─ routes/              +layout.ts（ssr=false）、各ウィンドウの画面
│  └─ lib/                 テーマ・i18n・api・app（ウィンドウ・メニュー）・stores・format・components
├─ Cargo.toml              Cargoワークスペース（src-tauri、crates/*）
├─ src-tauri/              アプリ本体（Rust・Tauri 2）。tauri.conf.json、capabilities/、icons/、binaries/（ビルドした補助プロセス。リポジトリに含めない）
├─ crates/
│  ├─ esp_latch_protocol/  本体と補助プロセスが共有する要求・応答の型
│  └─ esp_latch_helper/    root権限で動く補助プロセス（バイナリ名 esp-latch-helper）
├─ scripts/               checkコマンドと検査のスクリプト、疑似ロケール、補助プロセスのビルド（build_helper.sh）
├─ messages/              辞書（ja・en・zh-Hans。jaが正本）
├─ project.inlang/        Paraglide（inlang）の設定
├─ docs/brand/             ブランドの資料（brand.md・アイコンのマスターSVG）。roadmap/specs/の最新版のコピー
└─ roadmap/                企画と開発の記録
```

- SvelteKit 3では`svelte.config.js`を使いません。SvelteKitの設定（アダプターなど）は`vite.config.ts`の`sveltekit()`に書きます
- `roadmap/`：企画と開発の記録です。`specs/`にプランアセット（スペックシート・モック・トークン・ブランドの資料）の最新版、`planning_dialog/`に設計（imple）・実装の手順（tasks）・検証の手順（walk）、`development/`に修正と仕様変更の記録、`timeline.md`にプランアセットの版とリリースの時系列を置きます。`archived/`（旧版）はリポジトリに含めません
- `docs/brand/`：アプリのアイコンや見た目を作るときに参照するブランドの資料です。`roadmap/specs/`の最新版をコピーしたもので、直接編集しません

## デザインシステムとテーマ

- [Carbon Design System](https://carbondesignsystem.com/) v11のWeb Components版（`@carbon/web-components` 2.x）を使います。使う部品は`src/lib/components/carbon.ts`で登録します。ほかのデザインシステムの部品は混ぜません
- テーマは`src/lib/theme/`にあり、次の順に読み込みます（`src/lib/theme/index.ts`）
  1. `carbon_theme.scss`：Carbonの土台（ライトはg10、ダークはg100）を`<html data-theme>`に当てます。値は`@carbon/styles`から出力し、ファイルには書きません
  2. `tokens.css`：ブランドトークン。`roadmap/specs/`の最新の`esp_latch_tokens_v*.css`のコピーで、直接編集しません。Carbonの値を上書きし、Carbonにない値（書体の切り替え、角丸、マウント状態や帯グラフの色）を`--esp-*`で持ちます
  3. `fonts.css`：IBM Plex Sans・Sans JP・Monoを`@ibm/plex-*`のパッケージから同梱します（CDNを使いません）
  4. `base.css`：アプリ全体の基本
- 部品の色・余白・書体は、`--cds-*`と`--esp-*`だけを参照します。色や`px`を直接書きません
- 外観（システム・ライト・ダーク）は`src/lib/theme/theme.ts`の`applyTheme`で切り替えます
- Carbonのパッケージには、インストール時に利用状況を送る`postinstall`（`ibmtelemetry`）があります。Bunは既定でこれを実行しません。`bun pm trust`で許可しないでください（このアプリはネットワークに接続しない方針です）

## 国際化（i18n）

- 対応言語は日本語（原文）・英語・中国語簡体字（`zh-Hans`）です。[Paraglide JS](https://inlang.com/m/gerre34r/library-inlang-paraglideJs) 2を使います
- 辞書は`messages/{ja,en,zh-Hans}.json`で、`messages/ja.json`が文言の正本です。設定は`project.inlang/settings.json`にあります。メッセージの形式のプラグインは`node_modules`から読み込みます（ビルド時もCDNに接続しません）
- 辞書から`src/lib/paraglide/`にメッセージ関数が生成されます（開発サーバーとビルドで自動。リポジトリには含めません）。コードでは`import { m } from '#lib/i18n/index.js'`として`m['toolbar.refresh']()`のように引きます
- キーは機能ごとの階層をピリオドで区切り、各階層はスネークケースにします（例：`toolbar.refresh`）。JSONでは入れ子のオブジェクトで書きます。画面IDはキーに使いません
- 可変部分はプレースホルダ（`{count}`）で書き、文字列を連結して文を組み立てません。数で形が変わる文言は、すべての言語で複数形の形（`countPlural=one`・`countPlural=other`）にそろえます
- 新しいキーを足すときは、en・zh-Hansの訳案も同時に書きます
- 日付・時刻・数値・容量は、`src/lib/format/`の関数（`Intl`）で整形します
- ロケールは、設定の言語 → 「システムに合わせる」ならOSの優先言語（Rustのコマンド`system_locales`で取得）→ どれにも合わなければ日本語、の順で決めます（`src/lib/i18n/`）。ロケールが変わると、`<html lang>`を付け替え、画面を作り直します。書体は`tokens.css`の`:lang()`で切り替わります
- Carbonの部品が持つ文言（閉じるボタンのラベルなど）は、部品の既定の英語に任せず、辞書から属性で渡します（例：`<cds-modal-close-button close-button-label={m['modal.close']()}>`）

### 疑似ロケール

日本語の文言を3〜4割伸ばして`［］`で囲んだ表示で、直書きの取りこぼしと、長さによる崩れを確かめられます。開発サーバーでだけ有効です。

```sh
ESP_LATCH_PSEUDO_LOCALE=1 bun run tauri dev
```
