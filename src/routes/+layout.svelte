<script lang="ts">
	import '#lib/components/carbon.js';
	import { applyTheme } from '#lib/theme/index.js';
	import { applyLanguage, i18n } from '#lib/i18n/index.js';
	import { syncWindowTheme } from '#lib/app/windows.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	// 設定（language・theme）はPhase 2で読み込む。それまではシステムに従う
	applyTheme('system', (_theme, preference) => void syncWindowTheme(preference));
	const ready = applyLanguage('system');
</script>

{#await ready then}
	<!-- ロケールが変わったら画面を作り直し、すべての文言を引き直す -->
	{#key i18n.locale}
		{@render children()}
	{/key}
{/await}
