// 辞書からメッセージ関数（src/lib/paraglide/）を生成する。開発サーバーとビルドではViteのプラグインが行うため、
// 型検査やテストをViteの外で動かすとき（checkコマンド）に使う
import { compile } from '@inlang/paraglide-js';
import { paraglideOptions } from './lib/paraglide_options.ts';

await compile({ ...paraglideOptions, strategy: [...paraglideOptions.strategy] });
