// 外観（システム・ライト・ダーク）の切り替え。<html data-theme>を付け替え、
// carbon_theme.scss（g10・g100の土台）とtokens.css（ブランドの上書き）を切り替える。

export type ThemePreference = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';

const DARK_QUERY = '(prefers-color-scheme: dark)';

export function resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme {
	if (preference === 'system') return systemPrefersDark ? 'dark' : 'light';
	return preference;
}

let stopWatching: (() => void) | undefined;

/**
 * 外観を適用する。`system`のときはOSの外観の変化に追従する。
 * `onResolved`には、実際に当てた外観を渡す（ウィンドウのタイトルバーをそろえるため）。
 */
export function applyTheme(
	preference: ThemePreference,
	onResolved?: (theme: ResolvedTheme, preference: ThemePreference) => void
): void {
	stopWatching?.();
	stopWatching = undefined;

	const media = window.matchMedia(DARK_QUERY);
	const apply = () => {
		const theme = resolveTheme(preference, media.matches);
		document.documentElement.dataset.theme = theme;
		onResolved?.(theme, preference);
	};

	apply();
	if (preference === 'system') {
		media.addEventListener('change', apply);
		stopWatching = () => media.removeEventListener('change', apply);
	}
}
