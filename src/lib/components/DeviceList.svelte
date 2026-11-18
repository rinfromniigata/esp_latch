<script lang="ts">
	// サイドバー：見出しと台数、絞り込み、デバイスの一覧（読み込み中はスケルトン、該当なしは空の状態）
	import { m } from '#lib/i18n/index.js';
	import { deviceStore, selectionStore } from '#lib/stores/index.js';
	import DeviceListItem from './DeviceListItem.svelte';
	import FilterSwitcher from './FilterSwitcher.svelte';

	const total = $derived(deviceStore.devices.length);
	const shown = $derived(selectionStore.visible.length);
	const count = $derived(
		selectionStore.filter === 'all'
			? m['device_list.count']({ count: total })
			: m['device_list.count_filtered']({ shown, total })
	);
</script>

<div class="head">
	<div class="title">
		<span>{m['device_list.title']()}</span>
		{#if !deviceStore.loading}<span class="count">{count}</span>{/if}
	</div>
	<FilterSwitcher value={selectionStore.filter} onchange={(filter) => selectionStore.setFilter(filter)} />
</div>

<div class="list" role="list" aria-busy={deviceStore.loading}>
	{#if deviceStore.loading}
		{#each [0, 1, 2, 3] as row (row)}
			<div class="skeleton" role="listitem">
				<cds-skeleton-placeholder class="skeleton-icon"></cds-skeleton-placeholder>
				<div class="skeleton-text">
					<cds-skeleton-text></cds-skeleton-text>
					<cds-skeleton-text></cds-skeleton-text>
				</div>
			</div>
		{/each}
	{:else if selectionStore.visible.length === 0}
		<p class="empty">{m['empty.no_match.title']()}</p>
	{:else}
		{#each selectionStore.visible as device (device.id)}
			<div role="listitem">
				<DeviceListItem
					{device}
					selected={device.id === selectionStore.selectedId}
					onselect={(id) => selectionStore.select(id)}
				/>
			</div>
		{/each}
	{/if}
</div>

<style>
	.head {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-04);
		padding: var(--cds-spacing-05) var(--cds-spacing-05) var(--cds-spacing-04);
	}

	.title {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		font-size: var(--cds-body-compact-01-font-size);
		font-weight: 600;
	}

	.count {
		font-size: var(--cds-label-01-font-size);
		font-weight: 400;
		color: var(--cds-text-secondary);
	}

	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}

	.skeleton {
		display: flex;
		gap: var(--cds-spacing-04);
		padding: var(--cds-spacing-04) var(--cds-spacing-05);
		border-top: 1px solid var(--cds-border-subtle-01);
	}

	.skeleton-icon {
		flex: none;
		width: 1.5rem;
		height: 1.5rem;
	}

	.skeleton-text {
		flex: 1;
		min-width: 0;
	}

	.empty {
		margin: 0;
		padding: var(--cds-spacing-07) var(--cds-spacing-05);
		text-align: center;
		font-size: var(--cds-body-compact-01-font-size);
		color: var(--cds-text-secondary);
	}
</style>
