<script lang="ts">
	// メインウィンドウ（S01）の骨格。一覧と詳細はPhase 3で実装する
	import { onMount } from 'svelte';
	import Toolbar from '#lib/components/Toolbar.svelte';
	import { installAppMenu } from '#lib/app/menu.js';
	import { retitleCurrentWindow } from '#lib/app/windows.js';

	// ロケールが変わると、レイアウトの{#key}でこのページが作り直され、メニューとタイトルも作り直す
	onMount(() => {
		void installAppMenu();
		void retitleCurrentWindow('main');
	});
</script>

<div class="window">
	<Toolbar />
	<main class="body"></main>
</div>

<style>
	.window {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.body {
		flex: 1;
		min-height: 0;
	}
</style>
