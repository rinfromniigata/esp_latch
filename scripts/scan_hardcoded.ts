// 色・サイズの直書きの検出（imple 9章の7）：src/ の .svelte・.css・.scss で、
// 値の正本（tokens.css・carbon_theme.scss）以外に、色（#rgb・rgb()・hsl()など）や px の直書きがないかを調べる。
// 例外は scripts/hardcoded_allowlist.json に、ファイル（すべてのファイルなら "*"）・値・理由を書いて許可する。
import { readFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { listFiles } from './lib/files.ts';

/** 値の正本なので検査しないファイル */
const SOURCES_OF_VALUES = ['src/lib/theme/tokens.css', 'src/lib/theme/carbon_theme.scss'];

const PATTERNS: { name: string; regex: RegExp }[] = [
	{ name: '色', regex: /#[0-9a-fA-F]{3,8}\b/g },
	{ name: '色', regex: /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch)\(/g },
	{ name: 'サイズ', regex: /(?<![\w-])-?\d*\.?\d+px\b/g }
];

export type Finding = { line: number; kind: string; value: string };

/** CSSのコメントを、行の位置を保ったまま空白に置き換える */
function stripCssComments(css: string): string {
	return css.replace(/\/\*[\s\S]*?\*\//g, (comment) => comment.replace(/[^\n]/g, ' '));
}

/** CSS（またはSCSS）の本文から直書きを探す。offsetLineは、本文がファイルの何行目から始まるか */
export function scanCss(css: string, offsetLine = 1): Finding[] {
	const findings: Finding[] = [];
	const lines = stripCssComments(css)
		.split('\n')
		.map((line) => line.replace(/(^|[^:])\/\/.*$/, '$1'));
	lines.forEach((line, index) => {
		for (const { name, regex } of PATTERNS) {
			for (const match of line.matchAll(regex)) {
				findings.push({ line: offsetLine + index, kind: name, value: match[0] });
			}
		}
	});
	return findings;
}

/** .svelte の <style> の中と、style 属性・style: 指令の値を調べる */
export function scanSvelte(source: string): Finding[] {
	const findings: Finding[] = [];
	const lineAt = (offset: number) => source.slice(0, offset).split('\n').length;
	for (const match of source.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)) {
		const start = (match.index ?? 0) + match[0].indexOf(match[1]);
		findings.push(...scanCss(match[1], lineAt(start)));
	}
	for (const match of source.matchAll(/\sstyle(?::[\w-]+)?=(?:"([^"]*)"|'([^']*)'|\{([^}]*)\})/g)) {
		const value = match[1] ?? match[2] ?? match[3] ?? '';
		for (const finding of scanCss(value)) {
			findings.push({ ...finding, line: lineAt(match.index ?? 0) });
		}
	}
	return findings;
}

type AllowEntry = { file: string; value: string; reason: string };

function main(): number {
	const root = resolve(import.meta.dirname, '..');
	const allow = JSON.parse(readFileSync(resolve(root, 'scripts/hardcoded_allowlist.json'), 'utf8')) as AllowEntry[];
	const errors: string[] = [];

	for (const file of listFiles(resolve(root, 'src'), ['.svelte', '.css', '.scss'])) {
		const rel = relative(root, file);
		if (SOURCES_OF_VALUES.includes(rel) || rel.startsWith('src/lib/paraglide/')) continue;
		const source = readFileSync(file, 'utf8');
		const findings = file.endsWith('.svelte') ? scanSvelte(source) : scanCss(source);
		for (const finding of findings) {
			if (allow.some((entry) => (entry.file === '*' || entry.file === rel) && entry.value === finding.value)) continue;
			errors.push(`${finding.kind}の直書きです：${rel}:${finding.line}  ${finding.value}`);
		}
	}

	if (errors.length > 0) {
		console.error(errors.join('\n'));
		console.error('\n--cds-*・--esp-* のトークンを参照してください。やむを得ない場合は scripts/hardcoded_allowlist.json に理由とともに書きます');
		return 1;
	}
	return 0;
}

if (import.meta.main) process.exit(main());
