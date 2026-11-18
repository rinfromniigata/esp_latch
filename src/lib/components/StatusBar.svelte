<script lang="ts">
	// ステータスバー：状態（読み込み中・認証待ち・準備完了）、管理者権限で実行中の印、直前のコマンド
	import { m } from '#lib/i18n/index.js';
	import { espCount } from '#lib/devices/model.js';
	import { deviceStore, logStore, privilegedStore, settingsStore } from '#lib/stores/index.js';
	import Icon from './Icon.svelte';
	import * as icons from './icons';

	const state = $derived(
		privilegedStore.status === 'waiting_auth'
			? m['status.waiting_auth']()
			: deviceStore.loading
				? m['status.loading']()
				: m['status.ready']({
						count: deviceStore.devices.length,
						esp: deviceStore.devices.filter((device) => espCount(device) > 0).length
					})
	);
	const command = $derived(settingsStore.settings.show_command ? logStore.last?.command : undefined);
</script>

<footer class="statusbar">
	<span class="state" role="status">{state}</span>
	{#if privilegedStore.status === 'running'}
		<span class="privileged"><Icon icon={icons.lock} size="0.75rem" />{m['status.privileged']()}</span>
	{/if}
	{#if command}
		<span class="command" title={command}>
			<span class="label">{m['status.last_command']()}</span>
			<code>{command}</code>
		</span>
	{/if}
</footer>

<style>
	.statusbar {
		flex: none;
		display: flex;
		align-items: center;
		gap: var(--cds-spacing-05);
		height: 1.75rem;
		padding: 0 var(--cds-spacing-05);
		box-sizing: border-box;
		background-color: var(--cds-layer-01);
		border-top: 1px solid var(--cds-border-subtle-01);
		color: var(--cds-text-secondary);
		font-size: var(--cds-label-01-font-size);
	}

	.state {
		white-space: nowrap;
	}

	.privileged {
		display: inline-flex;
		align-items: center;
		gap: var(--cds-spacing-02);
		color: var(--esp-state-mounted);
		white-space: nowrap;
	}

	.command {
		flex: 1;
		min-width: 0;
		display: flex;
		justify-content: flex-end;
		gap: var(--cds-spacing-03);
	}

	.label {
		flex: none;
	}

	code {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-family: var(--esp-font-mono);
	}
</style>
