<script lang="ts">
	import '#lib/components/carbon.js';
	import { applyTheme } from '#lib/theme/index.js';
	import { applyLanguage, i18n } from '#lib/i18n/index.js';
	import { syncWindowTheme } from '#lib/app/windows.js';
	import { settingsStore } from '#lib/stores/index.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	// 設定を読み込んでから最初の画面を出す。以降は、設定の変更（settings-changed）に合わせて外観と言語を切り替える
	const ready = settingsStore.start().then((settings) => applyLanguage(settings.language));

	$effect(() => {
		applyTheme(settingsStore.settings.theme, (_theme, preference) => void syncWindowTheme(preference));
	});

	$effect(() => {
		void applyLanguage(settingsStore.settings.language);
	});
</script>

{#await ready then}
	<!-- ロケールが変わったら画面を作り直し、すべての文言を引き直す -->
	{#key i18n.locale}
		{@render children()}
	{/key}
{/await}
