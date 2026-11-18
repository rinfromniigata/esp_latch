// i18nの自動検査（imple 9章の5）：ロケール間のキーの過不足、プレースホルダの一致、複数形の形、
// 未使用キー、.svelte・.ts内の文字列の直書き。問題があれば一覧を出して終了コード1で終わる。
import { readFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { parse } from 'svelte/compiler';
import ts from 'typescript';
import { placeholders, readMessages, readSettings, type Message } from './lib/messages.ts';
import { listFiles } from './lib/files.ts';

// ---- ロケール間の検査 -------------------------------------------------------

export function pluralCategories(locale: string): string[] {
	return new Intl.PluralRules(locale).resolvedOptions().pluralCategories;
}

function variantSelectors(message: Message): string {
	return message.kind === 'simple'
		? ''
		: JSON.stringify(message.variants.map((variant) => variant.selectors ?? []));
}

/** 複数形のセレクター（local xPlural = x: plural）の名前を返す */
function pluralSelectorNames(message: Message): Set<string> {
	const names = new Set<string>();
	if (message.kind !== 'variants') return names;
	for (const variant of message.variants) {
		for (const declaration of variant.declarations ?? []) {
			const match = /^local\s+(\w+)\s*=\s*\w+\s*:\s*plural\b/.exec(declaration);
			if (match) names.add(match[1]);
		}
	}
	return names;
}

export function checkLocales(
	messagesByLocale: Map<string, Map<string, Message>>,
	baseLocale: string,
	categoriesOf: (locale: string) => string[] = pluralCategories
): string[] {
	const errors: string[] = [];
	const base = messagesByLocale.get(baseLocale);
	if (!base) return [`原文の辞書（${baseLocale}）がありません`];

	for (const [locale, messages] of messagesByLocale) {
		if (locale !== baseLocale) {
			for (const key of base.keys()) {
				if (!messages.has(key)) errors.push(`${locale}: キーがありません：${key}`);
			}
			for (const key of messages.keys()) {
				if (!base.has(key)) errors.push(`${locale}: 原文（${baseLocale}）にないキーです：${key}`);
			}
		}

		for (const [key, message] of messages) {
			const baseMessage = base.get(key);
			if (!baseMessage || locale === baseLocale) continue;

			const expected = [...placeholders(baseMessage)].sort().join(', ');
			const actual = [...placeholders(message)].sort().join(', ');
			if (expected !== actual) {
				errors.push(`${locale}: プレースホルダが原文と違います：${key}（原文 {${expected}}、${locale} {${actual}}）`);
			}
			if (message.kind !== baseMessage.kind || variantSelectors(message) !== variantSelectors(baseMessage)) {
				errors.push(`${locale}: 文言の形（単純・複数形）が原文と違います：${key}`);
			}
		}

		// 複数形：そのロケールの複数形の区分（Intl.PluralRules）をすべて持つこと
		for (const [key, message] of messages) {
			if (message.kind !== 'variants') continue;
			for (const selector of pluralSelectorNames(message)) {
				for (const variant of message.variants) {
					const has = new Set(
						Object.keys(variant.match)
							.map((condition) => new RegExp(`(?:^|,)\\s*${selector}=([\\w*]+)`).exec(condition)?.[1])
							.filter((value): value is string => value !== undefined)
					);
					if (has.has('*')) continue;
					const missing = categoriesOf(locale).filter((category) => !has.has(category));
					if (missing.length > 0) {
						errors.push(`${locale}: 複数形の区分が足りません：${key}（${missing.join('・')}）`);
					}
				}
			}
		}
	}
	return errors;
}

// ---- 未使用キー ---------------------------------------------------------------

/** ソースから、m['key']・m["key"]・m.key の形で参照しているキーを集める */
export function findUsedKeys(source: string): Set<string> {
	const used = new Set<string>();
	for (const match of source.matchAll(/\bm\[\s*(['"])([\w.]+)\1\s*\]/g)) used.add(match[2]);
	for (const match of source.matchAll(/\bm\.(\w+)\s*\(/g)) used.add(match[1]);
	return used;
}

// ---- 文字列の直書き ---------------------------------------------------------

export type Finding = { line: number; text: string };

const CJK = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\u3000-\u303f\uff01-\uff60]/u;
const LETTER = /\p{L}/u;
/** 利用者が読む文言を受け取る属性（Carbonの部品の *-label・*-text などを含む） */
const USER_FACING_ATTRIBUTE =
	/^(?:aria-(?:label|description|placeholder|roledescription|valuetext)|(?:.*-)?(?:label|text|title|heading|description|placeholder|alt|tooltip|caption|subtitle))$/i;

function lineOf(source: string, offset: number): number {
	return source.slice(0, offset).split('\n').length;
}

/** TypeScriptのコードから、和文・全角の文字を含む文字列リテラルを探す（コメントは対象外） */
export function findHardcodedInTs(code: string, offset = 0, whole = code): Finding[] {
	const findings: Finding[] = [];
	const file = ts.createSourceFile('x.ts', code, ts.ScriptTarget.Latest, true);
	const visit = (node: ts.Node) => {
		if (
			ts.isStringLiteral(node) ||
			ts.isNoSubstitutionTemplateLiteral(node) ||
			ts.isTemplateHead(node) ||
			ts.isTemplateMiddle(node) ||
			ts.isTemplateTail(node)
		) {
			if (CJK.test(node.text)) {
				findings.push({ line: lineOf(whole, offset + node.getStart(file)), text: node.text });
			}
		}
		ts.forEachChild(node, visit);
	};
	visit(file);
	return findings;
}

type SvelteNode = { type: string; start: number; end: number; [key: string]: unknown };

function isNode(value: unknown): value is SvelteNode {
	return typeof value === 'object' && value !== null && typeof (value as SvelteNode).type === 'string';
}

/** .svelteのマークアップの文字、利用者向けの属性の文字、式とスクリプトの中の和文の文字列を探す */
export function findHardcodedInSvelte(source: string): Finding[] {
	const findings: Finding[] = [];
	const ast = parse(source, { modern: true });

	for (const script of [ast.instance, ast.module]) {
		if (!script) continue;
		const content = script.content as unknown as SvelteNode;
		const code = source.slice(content.start, content.end);
		findings.push(...findHardcodedInTs(code, content.start, source));
	}

	const scanExpression = (node: SvelteNode) => {
		const code = source.slice(node.start, node.end);
		findings.push(...findHardcodedInTs(`(${code})`, node.start - 1, source));
	};

	const walk = (value: unknown): void => {
		if (Array.isArray(value)) {
			value.forEach(walk);
			return;
		}
		if (!isNode(value)) return;

		if (value.type === 'Text') {
			const text = String(value.data ?? '').trim();
			if (text && LETTER.test(text)) findings.push({ line: lineOf(source, value.start), text });
			return;
		}
		if (value.type === 'Comment') return;
		if (value.type === 'Attribute') {
			const parts = Array.isArray(value.value) ? value.value : [value.value];
			for (const part of parts) {
				if (!isNode(part)) continue;
				if (part.type === 'Text') {
					const text = String(part.data ?? '').trim();
					if (USER_FACING_ATTRIBUTE.test(String(value.name)) && LETTER.test(text)) {
						findings.push({ line: lineOf(source, part.start), text: `${String(value.name)}="${text}"` });
					}
				} else if (part.type === 'ExpressionTag' && isNode(part.expression)) {
					scanExpression(part.expression);
				}
			}
			return;
		}
		if (value.type === 'ExpressionTag' && isNode(value.expression)) {
			scanExpression(value.expression);
			return;
		}
		for (const [key, child] of Object.entries(value)) {
			if (key === 'parent' || key === 'metadata') continue;
			walk(child);
		}
	};
	walk(ast.fragment);
	return findings;
}

// ---- 実行 -------------------------------------------------------------------

type AllowEntry = { file: string; text: string; reason: string };
type Pending = { keys: string[] };

function main(): number {
	const root = resolve(import.meta.dirname, '..');
	const errors: string[] = [];

	const settings = readSettings(root);
	const messagesByLocale = new Map(settings.locales.map((locale) => [locale, readMessages(root, locale)]));
	errors.push(...checkLocales(messagesByLocale, settings.baseLocale));

	const sources = listFiles(resolve(root, 'src'), ['.svelte', '.ts']).filter(
		(file) => !file.includes('/src/lib/paraglide/') && !/\.(spec|test)\.ts$/.test(file)
	);

	// 未使用キー（画面がまだない文言は i18n_pending.json に置き、使い始めたら外す）
	const pending = JSON.parse(readFileSync(resolve(root, 'scripts/i18n_pending.json'), 'utf8')) as Pending;
	const pendingKeys = new Set(pending.keys);
	const used = new Set<string>();
	for (const file of sources) for (const key of findUsedKeys(readFileSync(file, 'utf8'))) used.add(key);
	const baseKeys = messagesByLocale.get(settings.baseLocale)?.keys() ?? [];
	for (const key of baseKeys) {
		if (!used.has(key) && !pendingKeys.has(key)) errors.push(`未使用のキーです：${key}`);
		if (used.has(key) && pendingKeys.has(key)) {
			errors.push(`使い始めたキーが scripts/i18n_pending.json に残っています：${key}`);
		}
	}
	const known = new Set(messagesByLocale.get(settings.baseLocale)?.keys() ?? []);
	for (const key of pendingKeys) {
		if (!known.has(key)) errors.push(`scripts/i18n_pending.json に辞書にないキーがあります：${key}`);
	}

	// 直書き
	const allow = JSON.parse(readFileSync(resolve(root, 'scripts/i18n_allowlist.json'), 'utf8')) as AllowEntry[];
	for (const file of sources) {
		const source = readFileSync(file, 'utf8');
		const rel = relative(root, file);
		const findings = file.endsWith('.svelte') ? findHardcodedInSvelte(source) : findHardcodedInTs(source);
		for (const finding of findings) {
			if (allow.some((entry) => entry.file === rel && entry.text === finding.text)) continue;
			errors.push(`文字列の直書きです：${rel}:${finding.line}  ${finding.text}`);
		}
	}

	if (errors.length > 0) {
		console.error(errors.join('\n'));
		console.error(`\ni18nの検査で${errors.length}件の問題が見つかりました`);
		return 1;
	}
	return 0;
}

if (import.meta.main) process.exit(main());
