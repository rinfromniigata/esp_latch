import { describe, expect, it } from 'vitest';
import { matchLocale, resolveLocale } from './resolve';

describe('matchLocale', () => {
	it('日本語・英語は地域を問わない', () => {
		expect(matchLocale('ja')).toBe('ja');
		expect(matchLocale('ja-JP')).toBe('ja');
		expect(matchLocale('en-GB')).toBe('en');
		expect(matchLocale('en_US')).toBe('en');
	});

	it('中国語は簡体字だけに対応する', () => {
		expect(matchLocale('zh-Hans')).toBe('zh-Hans');
		expect(matchLocale('zh-Hans-CN')).toBe('zh-Hans');
		expect(matchLocale('zh-CN')).toBe('zh-Hans');
		expect(matchLocale('zh-SG')).toBe('zh-Hans');
		expect(matchLocale('zh-Hant-TW')).toBeUndefined();
		expect(matchLocale('zh-TW')).toBeUndefined();
		expect(matchLocale('zh')).toBeUndefined();
	});

	it('対応しない言語はundefined', () => {
		expect(matchLocale('fr-FR')).toBeUndefined();
	});
});

describe('resolveLocale', () => {
	it('設定で選んだ言語を優先する', () => {
		expect(resolveLocale('en', ['ja-JP'])).toBe('en');
	});

	it('systemならOSの優先言語を順に見て、最初に対応するものを使う', () => {
		expect(resolveLocale('system', ['fr-FR', 'zh-Hans-CN', 'en-US'])).toBe('zh-Hans');
	});

	it('どれにも対応しなければ日本語', () => {
		expect(resolveLocale('system', ['fr-FR', 'zh-Hant-TW'])).toBe('ja');
		expect(resolveLocale('system', [])).toBe('ja');
	});
});
