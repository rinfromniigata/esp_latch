<script lang="ts">
	// メインウィンドウ（S01）：タイトルバー・ツールバー、サイドバーと詳細、ステータスバー
	import { onMount } from 'svelte';
	import TitleBar from '#lib/components/TitleBar.svelte';
	import StatusBar from '#lib/components/StatusBar.svelte';
	import DeviceList from '#lib/components/DeviceList.svelte';
	import DeviceDetail from '#lib/components/DeviceDetail.svelte';
	import { installAppMenu } from '#lib/app/menu.js';
	import { retitleCurrentWindow } from '#lib/app/windows.js';
	import { deviceStore, logStore, privilegedStore, selectionStore } from '#lib/stores/index.js';

	// ロケールが変わると、レイアウトの{#key}でこのページが作り直され、メニューとタイトルも作り直す。
	// ストアは1回だけ開始し、作り直しても状態を引き継ぐ
	onMount(() => {
		void installAppMenu();
		void retitleCurrentWindow('main');
		void Promise.all([deviceStore.start(), logStore.start(), privilegedStore.start()]).then(() =>
			selectionStore.initialize()
		);
	});
</script>

<div class="window">
	<TitleBar />
	<div class="body">
		<aside class="sidebar"><DeviceList /></aside>
		<main class="detail"><DeviceDetail /></main>
	</div>
	<StatusBar />
</div>

<style>
	.window {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.body {
		flex: 1;
		display: flex;
		min-height: 0;
	}

	.sidebar {
		flex: none;
		width: 18.75rem;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background-color: var(--cds-layer-01);
		border-right: 1px solid var(--cds-border-subtle-01);
	}

	.detail {
		flex: 1;
		min-width: 0;
		overflow: auto;
		padding: var(--cds-spacing-06);
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-06);
	}
</style>
