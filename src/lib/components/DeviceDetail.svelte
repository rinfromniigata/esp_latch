<script lang="ts">
	// メインウィンドウの詳細：読み込み中はスケルトン、該当なし・未選択は空の状態、
	// 選んだデバイスは見出し・帯グラフ・ESPの節・その他のパーティションの表
	import { m } from '#lib/i18n/index.js';
	import { displayNames, espPartitions, isBootEsp } from '#lib/devices/model.js';
	import { filterLabel } from '#lib/devices/labels.js';
	import { deviceStore, espStore, selectionStore } from '#lib/stores/index.js';
	import DetailHeader from './DetailHeader.svelte';
	import EmptyState from './EmptyState.svelte';
	import EspTile from './EspTile.svelte';
	import PartitionBand from './PartitionBand.svelte';
	import PartitionTable from './PartitionTable.svelte';
	import * as icons from './icons';

	const device = $derived(selectionStore.selected);
	const esps = $derived(device ? espPartitions(device) : []);

	// 選んだデバイスが変わる・一覧が更新されるたびに、マウント中のESPの詳細（ブートローダー・容量）を読み直す
	$effect(() => {
		void espStore.sync(esps);
	});
</script>

{#if deviceStore.loading}
	<div class="skeleton" aria-hidden="true">
		<cds-skeleton-placeholder class="s-head"></cds-skeleton-placeholder>
		<cds-skeleton-placeholder class="s-band"></cds-skeleton-placeholder>
		<cds-skeleton-placeholder class="s-tile"></cds-skeleton-placeholder>
	</div>
{:else if selectionStore.visible.length === 0}
	<EmptyState
		icon={icons.internalDisk}
		title={m['empty.no_match.title']()}
		body={m['empty.no_match.body']({ filter: filterLabel('all') })}
	>
		<cds-button kind="tertiary" size="md" onclick={() => selectionStore.setFilter('all')}>
			{filterLabel('all')}
		</cds-button>
	</EmptyState>
{:else if !device}
	<EmptyState icon={icons.internalDisk} title={m['empty.select.title']()} body={m['empty.select.body']()} />
{:else}
	<DetailHeader {device} />
	<PartitionBand {device} />
	<section>
		<h3 class="section-title">{m['esp.section_title']()}</h3>
		{#if esps.length === 0}
			<div class="no-esp">
				<h4>{m['empty.no_esp.title']()}</h4>
				<p>{m['empty.no_esp.body']({ device: displayNames(device).main })}</p>
			</div>
		{:else}
			<div class="esp-grid">
				{#each esps as partition (partition.id)}
					<EspTile {partition} boot={deviceStore.snapshot !== null && isBootEsp(partition, deviceStore.snapshot)} />
				{/each}
			</div>
		{/if}
	</section>
	<PartitionTable {device} />
{/if}

<style>
	.skeleton {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-06);
	}

	.skeleton :global(.s-head) {
		width: 50%;
		height: 2.5rem;
	}

	.skeleton :global(.s-band) {
		width: 100%;
		height: 1.75rem;
	}

	.skeleton :global(.s-tile) {
		width: 20rem;
		height: 10rem;
	}

	.section-title {
		margin: 0 0 var(--cds-spacing-04);
		font-size: var(--cds-body-compact-01-font-size);
		font-weight: 600;
	}

	.esp-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(18.75rem, 1fr));
		gap: var(--cds-spacing-04);
	}

	.no-esp {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--cds-spacing-02);
		padding: var(--cds-spacing-05);
		text-align: center;
		background-color: var(--cds-layer-01);
		border: 1px solid var(--cds-border-subtle-01);
		border-radius: var(--esp-radius-md);
	}

	.no-esp h4 {
		margin: 0;
		font-size: var(--cds-body-compact-02-font-size);
		font-weight: 500;
	}

	.no-esp p {
		margin: 0;
		font-size: var(--cds-body-compact-01-font-size);
		color: var(--cds-text-secondary);
	}
</style>
