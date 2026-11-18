<script lang="ts">
	// macOSのタイトルバー周り。ウィンドウの枠は信号ボタンだけを残して（tauri.conf.jsonのOverlay）、
	// アプリ名とツールバーを同じ帯に並べる。帯の空いたところはウィンドウの移動に使う
	import { m } from '#lib/i18n/index.js';
	import Toolbar from './Toolbar.svelte';
</script>

<header class="titlebar" data-tauri-drag-region>
	<span class="name" data-tauri-drag-region>{m['app.name']()}</span>
	<span class="spacer" data-tauri-drag-region></span>
	<Toolbar />
</header>

<style>
	.titlebar {
		flex: none;
		display: flex;
		align-items: center;
		gap: var(--cds-spacing-04);
		/* 左は信号ボタン（閉じる・しまう・拡大）の幅を空ける */
		padding: 0 var(--cds-spacing-04) 0 4.75rem;
		height: 2.75rem;
		box-sizing: border-box;
		background-color: var(--cds-layer-01);
		border-bottom: 1px solid var(--cds-border-subtle-01);
		user-select: none;
		-webkit-user-select: none;
	}

	.name {
		font-weight: 600;
		font-size: var(--cds-body-compact-01-font-size);
		white-space: nowrap;
	}

	.spacer {
		flex: 1;
		align-self: stretch;
	}
</style>
