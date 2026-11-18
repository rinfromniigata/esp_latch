// ロケールの決め方（imple 8章）：設定のlanguage → systemなら、OSの優先言語 → どれでもなければja
import { baseLocale, locales, type Locale } from '#lib/paraglide/runtime.js';

export type LanguagePreference = 'system' | Locale;

/** OSの言語タグ1つを、対応するロケールに対応づける。対応しなければundefined */
export function matchLocale(tag: string): Locale | undefined {
	const lower = tag.toLowerCase().replaceAll('_', '-');
	const [language, ...rest] = lower.split('-');
	if (language === 'ja') return 'ja';
	if (language === 'en') return 'en';
	if (language === 'zh') {
		// 簡体字：zh-Hans、zh-CN、zh-SG（zh-Hans-CNなどを含む）。繁体字（zh-Hant、zh-TW、zh-HK）は対応しない
		if (rest.includes('hans') || rest.includes('cn') || rest.includes('sg')) {
			return rest.includes('hant') ? undefined : 'zh-Hans';
		}
	}
	return undefined;
}

export function resolveLocale(
	preference: LanguagePreference,
	systemLanguages: readonly string[]
): Locale {
	if (preference !== 'system') return preference;
	for (const tag of systemLanguages) {
		const locale = matchLocale(tag);
		if (locale) return locale;
	}
	return baseLocale;
}

export function isLocale(value: string): value is Locale {
	return (locales as readonly string[]).includes(value);
}
