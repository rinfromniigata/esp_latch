<script lang="ts">
	// メインウィンドウのツールバー（更新・操作ログ・設定）。タイトルバーの右に置く
	import { m } from '#lib/i18n/index.js';
	import { openAppWindow } from '#lib/app/windows.js';
	import { deviceStore } from '#lib/stores/index.js';
	import Icon from './Icon.svelte';
	import * as icons from './icons';
</script>

<div class="toolbar" role="toolbar" aria-label={m['app.name']()}>
	<cds-icon-button
		kind="ghost"
		size="md"
		align="bottom-end"
		disabled={deviceStore.loading}
		onclick={() => void deviceStore.refresh()}
	>
		<Icon icon={icons.refresh} />
		<span slot="tooltip-content">{m['toolbar.refresh']()}</span>
	</cds-icon-button>
	<cds-icon-button kind="ghost" size="md" align="bottom-end" onclick={() => void openAppWindow('log')}>
		<Icon icon={icons.log} />
		<span slot="tooltip-content">{m['toolbar.log']()}</span>
	</cds-icon-button>
	<cds-icon-button kind="ghost" size="md" align="bottom-end" onclick={() => void openAppWindow('settings')}>
		<Icon icon={icons.settings} />
		<span slot="tooltip-content">{m['toolbar.settings']()}</span>
	</cds-icon-button>
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		gap: var(--cds-spacing-02);
	}
</style>
