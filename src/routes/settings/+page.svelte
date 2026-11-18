<script lang="ts">
	// 設定のウィンドウ（S02）の骨格。中身はPhase 4で実装する
	import { onMount } from 'svelte';
	import { m } from '#lib/i18n/index.js';
	import { retitleCurrentWindow } from '#lib/app/windows.js';

	onMount(() => {
		void retitleCurrentWindow('settings');
	});
</script>

<main class="window">
	<h1 class="title">{m['settings.title']()}</h1>
</main>

<style>
	.window {
		height: 100%;
		padding: var(--cds-spacing-06);
		box-sizing: border-box;
	}

	.title {
		margin: 0;
		font-size: var(--cds-heading-03-font-size);
		font-weight: var(--cds-heading-03-font-weight);
		line-height: var(--cds-heading-03-line-height);
	}
</style>
