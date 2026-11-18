// 疑似ロケール：jaの文言を3〜4割伸ばし、［］で囲む。直書きの取りこぼしと、長さによる崩れを画面で確かめる。
// 開発時だけ使う（ESP_LATCH_PSEUDO_LOCALE=1 bun run tauri dev）。公開版の言語には含めない。
// 仕組み：jaの文言を疑似の文言に差し替えたinlangのプロジェクトを作り、Paraglideのコンパイル元をそちらに向ける。
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const PLACEHOLDER = /(\{[^}]*\})/;
const FILLER = '〜';
const STRETCH = 0.35;

/** 文言1つを疑似化する。プレースホルダ（{count}など）はそのまま残す */
export function pseudoize(text: string): string {
	const visibleLength = text
		.split(PLACEHOLDER)
		.filter((part) => !PLACEHOLDER.test(part))
		.join('').length;
	const padding = FILLER.repeat(Math.max(1, Math.ceil(visibleLength * STRETCH)));
	return `［${text}${padding}］`;
}

type Messages = { [key: string]: unknown };

/** 辞書のJSON全体を疑似化する（入れ子と、複数形のmatchの各値を含む） */
export function pseudoizeMessages(messages: Messages): Messages {
	const walk = (value: unknown, key: string): unknown => {
		if (key === '$schema') return value;
		if (typeof value === 'string') return pseudoize(value);
		if (Array.isArray(value)) {
			return value.map((variant: { match?: Record<string, string> }) => ({
				...variant,
				match: Object.fromEntries(
					Object.entries(variant.match ?? {}).map(([selector, text]) => [selector, pseudoize(text)])
				)
			}));
		}
		if (value && typeof value === 'object') {
			return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, walk(v, k)]));
		}
		return value;
	};
	return walk(messages, '') as Messages;
}

/** 疑似ロケールのinlangのプロジェクトを作り、そのパスを返す */
export function buildPseudoProject(root: string): string {
	const outDir = resolve(root, '.svelte-kit/pseudo_locale');
	rmSync(outDir, { recursive: true, force: true });
	mkdirSync(join(outDir, 'project.inlang'), { recursive: true });
	cpSync(resolve(root, 'messages'), join(outDir, 'messages'), { recursive: true });

	const ja = JSON.parse(readFileSync(resolve(root, 'messages/ja.json'), 'utf8')) as Messages;
	writeFileSync(join(outDir, 'messages/ja.json'), JSON.stringify(pseudoizeMessages(ja), null, '\t'));

	const settings = JSON.parse(readFileSync(resolve(root, 'project.inlang/settings.json'), 'utf8'));
	settings.modules = (settings.modules as string[]).map((module) =>
		module.startsWith('http') ? module : resolve(root, module)
	);
	writeFileSync(join(outDir, 'project.inlang/settings.json'), JSON.stringify(settings, null, '\t'));
	return join(outDir, 'project.inlang');
}
