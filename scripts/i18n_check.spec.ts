import { describe, expect, it } from 'vitest';
import { checkLocales, findHardcodedInSvelte, findHardcodedInTs, findUsedKeys } from './i18n_check';
import { flattenMessages } from './lib/messages';

const plural = (match: Record<string, string>) => [
	{
		declarations: ['input count', 'local countPlural = count: plural'],
		selectors: ['countPlural'],
		match
	}
];

const locales = (dicts: Record<string, Record<string, unknown>>) =>
	new Map(Object.entries(dicts).map(([locale, json]) => [locale, flattenMessages(json)]));

const categories = (locale: string) => (locale === 'en' ? ['one', 'other'] : ['other']);

describe('checkLocales', () => {
	it('そろっていれば問題なし', () => {
		const result = checkLocales(
			locales({
				ja: { a: { b: '{n}件' }, c: plural({ 'countPlural=other': '{count}台' }) },
				en: {
					a: { b: '{n} items' },
					c: plural({ 'countPlural=one': '{count} device', 'countPlural=other': '{count} devices' })
				}
			}),
			'ja',
			categories
		);
		expect(result).toEqual([]);
	});

	it('キーの過不足を見つける', () => {
		const result = checkLocales(locales({ ja: { a: 'あ', b: 'い' }, en: { a: 'A', c: 'C' } }), 'ja', categories);
		expect(result).toEqual(['en: キーがありません：b', 'en: 原文（ja）にないキーです：c']);
	});

	it('プレースホルダの違いを見つける', () => {
		const result = checkLocales(locales({ ja: { a: '{n}件' }, en: { a: '{count} items' } }), 'ja', categories);
		expect(result).toHaveLength(1);
		expect(result[0]).toContain('プレースホルダ');
	});

	it('複数形の形の違いと、区分の不足を見つける', () => {
		const result = checkLocales(
			locales({
				ja: { a: plural({ 'countPlural=other': '{count}個' }), b: plural({ 'countPlural=other': '{count}個' }) },
				en: { a: '{count} items', b: plural({ 'countPlural=other': '{count} items' }) }
			}),
			'ja',
			categories
		);
		expect(result).toEqual([
			'en: 文言の形（単純・複数形）が原文と違います：a',
			'en: 複数形の区分が足りません：b（one）'
		]);
	});
});

describe('findUsedKeys', () => {
	it('m[…] と m.… の参照を集める', () => {
		const keys = findUsedKeys(`m['toolbar.refresh']() + m["a.b"]({ n: 1 }) + m.title()`);
		expect([...keys].sort()).toEqual(['a.b', 'title', 'toolbar.refresh']);
	});
});

describe('findHardcodedInTs', () => {
	it('和文の文字列リテラルを見つけ、コメントは無視する', () => {
		const findings = findHardcodedInTs(`// 更新する\nconst a = 'ok';\nconst b = '更新';\nconst c = \`\${a}台\`;`);
		expect(findings.map((finding) => finding.text)).toEqual(['更新', '台']);
		expect(findings[0].line).toBe(3);
	});
});

describe('findHardcodedInSvelte', () => {
	it('マークアップの文字、利用者向けの属性、式の中の和文を見つける', () => {
		const source = [
			'<script lang="ts">',
			"\tconst label = '設定';",
			'</script>',
			'<!-- コメントは対象外 -->',
			'<div class="toolbar" title="Settings">',
			'\t<span>{m[\'toolbar.refresh\']()}</span>',
			'\t<p>Refresh</p>',
			"\t<cds-button close-button-label={'閉じる'}>{label}</cds-button>",
			'</div>'
		].join('\n');
		const findings = findHardcodedInSvelte(source).map((finding) => `${finding.line}:${finding.text}`);
		expect(findings).toEqual(['2:設定', '5:title="Settings"', '7:Refresh', '8:閉じる']);
	});
});
