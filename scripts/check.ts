// checkコマンド（bun run check）：型検査・lint・テスト（TypeScriptとRust）、i18n、コピーの一致、直書きの検出を
// 順に実行し、失敗した段階の出力だけを表示する（imple 9章）。CIも同じコマンドを呼ぶ。
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

type Stage = { name: string; commands: string[][] };

const STAGES: Stage[] = [
	{
		name: '準備（メッセージ関数の生成・SvelteKitの型の生成）',
		commands: [
			['bun', 'scripts/paraglide_compile.ts'],
			['bunx', 'svelte-kit', 'sync']
		]
	},
	{ name: '型検査（svelte-check）', commands: [['bunx', 'svelte-check', '--tsconfig', './tsconfig.json', '--fail-on-warnings']] },
	{ name: 'lint（eslint）', commands: [['bunx', 'eslint', '.', '--max-warnings', '0']] },
	{ name: 'テスト（vitest）', commands: [['bunx', 'vitest', 'run']] },
	{
		name: 'Rust（fmt・clippy・test）',
		commands: [
			['cargo', 'fmt', '--all', '--check'],
			['cargo', 'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'],
			['cargo', 'test', '--workspace']
		]
	},
	{ name: 'i18n（キー・プレースホルダ・複数形・未使用キー・直書き）', commands: [['bun', 'scripts/i18n_check.ts']] },
	{ name: 'コピーの一致（tokens.css・docs/brand/）', commands: [['bun', 'scripts/check_copies.ts']] },
	{ name: '色・サイズの直書き', commands: [['bun', 'scripts/scan_hardcoded.ts']] }
];

const root = resolve(import.meta.dirname, '..');
let failed = 0;

for (const stage of STAGES) {
	let output = '';
	let ok = true;
	for (const [command, ...args] of stage.commands) {
		const result = spawnSync(command, args, {
			cwd: root,
			encoding: 'utf8',
			env: { ...process.env, FORCE_COLOR: '1', CARGO_TERM_COLOR: 'always' },
			maxBuffer: 64 * 1024 * 1024
		});
		if (result.error || result.status !== 0) {
			output += `$ ${[command, ...args].join(' ')}\n`;
			output += result.error ? `${result.error.message}\n` : `${result.stdout}${result.stderr}`;
			ok = false;
			break;
		}
	}
	if (ok) {
		console.log(`✓ ${stage.name}`);
	} else {
		failed += 1;
		console.log(`✗ ${stage.name}\n${output.trimEnd()}\n`);
	}
}

if (failed > 0) {
	console.log(`\ncheckコマンド：${STAGES.length}段階のうち${failed}段階が失敗しました`);
	process.exit(1);
}
console.log('\ncheckコマンド：すべて通過しました');
