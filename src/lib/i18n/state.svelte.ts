// 現在のロケールの保持と適用。ロケールが変わると、ルートのレイアウトが{#key}で画面を作り直す
import { baseLocale, getLocale, setLocale, type Locale } from '#lib/paraglide/runtime.js';
import { systemLocales } from '#lib/api/commands.js';
import { applyCarbonFonts } from '#lib/theme/carbon_fonts.js';
import { resolveLocale, type LanguagePreference } from './resolve';

let current = $state<Locale>(getLocale());

export const i18n = {
	/** 現在のロケール。画面の作り直しやメニューの再構築のきっかけに使う */
	get locale(): Locale {
		return current;
	}
};

async function readSystemLanguages(): Promise<readonly string[]> {
	try {
		return await systemLocales();
	} catch {
		// Tauriの外（ブラウザだけでの開発）では、WebViewの言語で代用する
		return navigator.languages;
	}
}

/** 設定のlanguageを適用する。<html lang>も合わせ、tokens.cssの:lang()で書体を切り替える */
export async function applyLanguage(preference: LanguagePreference): Promise<Locale> {
	const systemLanguages = preference === 'system' ? await readSystemLanguages() : [];
	// 疑似ロケール（開発時だけ）は、jaの文言を差し替えて作るため、常にjaで表示する
	const locale = import.meta.env.ESP_LATCH_PSEUDO_LOCALE
		? baseLocale
		: resolveLocale(preference, systemLanguages);
	// 画面は{#key}で作り直すため、文書の再読み込みはしない
	setLocale(locale, { reload: false });
	document.documentElement.lang = locale;
	applyCarbonFonts(locale);
	current = locale;
	return locale;
}
