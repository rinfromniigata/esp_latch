// 辞書（messages/{locale}.json）の読み込みと、キーごとの平らな形への変換
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

export type Variant = {
	declarations?: string[];
	selectors?: string[];
	match: Record<string, string>;
};

/** キー1つ分の文言。単純な文字列か、複数形などの変化形を持つもの */
export type Message = { kind: 'simple'; text: string } | { kind: 'variants'; variants: Variant[] };

export function readSettings(root: string): { baseLocale: string; locales: string[] } {
	return JSON.parse(readFileSync(resolve(root, 'project.inlang/settings.json'), 'utf8'));
}

export function flattenMessages(json: Record<string, unknown>, prefix = ''): Map<string, Message> {
	const out = new Map<string, Message>();
	for (const [key, value] of Object.entries(json)) {
		if (key === '$schema') continue;
		const id = prefix ? `${prefix}.${key}` : key;
		if (typeof value === 'string') out.set(id, { kind: 'simple', text: value });
		else if (Array.isArray(value)) out.set(id, { kind: 'variants', variants: value as Variant[] });
		else if (value && typeof value === 'object') {
			for (const [k, v] of flattenMessages(value as Record<string, unknown>, id)) out.set(k, v);
		}
	}
	return out;
}

export function readMessages(root: string, locale: string): Map<string, Message> {
	const json = JSON.parse(readFileSync(resolve(root, `messages/${locale}.json`), 'utf8'));
	return flattenMessages(json);
}

/** 文言に現れるプレースホルダの変数名（{count}・{count: number …}の count） */
export function placeholders(message: Message): Set<string> {
	const texts =
		message.kind === 'simple'
			? [message.text]
			: message.variants.flatMap((variant) => Object.values(variant.match));
	const names = new Set<string>();
	for (const text of texts) {
		for (const match of text.matchAll(/(?<!\\)\{\s*([A-Za-z_][\w]*)\s*(?::[^}]*)?\}/g)) {
			names.add(match[1]);
		}
	}
	return names;
}
