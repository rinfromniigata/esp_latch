<script lang="ts">
	// メインウィンドウのツールバー（骨格）。見た目と更新の動作はPhase 3でモックに合わせて仕上げる
	import { m } from '#lib/i18n/index.js';
	import { openAppWindow } from '#lib/app/windows.js';
</script>

<div class="toolbar" role="toolbar">
	<cds-button kind="ghost" size="sm" disabled>{m['toolbar.refresh']()}</cds-button>
	<span class="spacer"></span>
	<cds-button kind="ghost" size="sm" onclick={() => openAppWindow('log')}>{m['toolbar.log']()}</cds-button>
	<cds-button kind="ghost" size="sm" onclick={() => openAppWindow('settings')}>
		{m['toolbar.settings']()}
	</cds-button>
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		gap: var(--cds-spacing-02);
		padding: var(--cds-spacing-02) var(--cds-spacing-04);
		background-color: var(--cds-layer-01);
		border-bottom: 1px solid var(--cds-border-subtle-01);
	}

	.spacer {
		flex: 1;
	}
</style>
