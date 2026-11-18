<script lang="ts">
	// その他のパーティションの表（Carbonのデータテーブル）。APFSのパーティションは、行を開くと、
	// その下にコンテナとボリュームを入れ子で見せる（列をそろえるため、同じ表の行として並べる）
	import { SvelteSet } from 'svelte/reactivity';
	import { m } from '#lib/i18n/index.js';
	import type { Device } from '#lib/api/types.js';
	import { formatBytes } from '#lib/format/index.js';
	import { EMPTY_VALUE } from '#lib/devices/labels.js';
	import { otherPartitions } from '#lib/devices/model.js';
	import Icon from './Icon.svelte';
	import * as icons from './icons';

	let { device }: { device: Device } = $props();

	const rows = $derived(otherPartitions(device));
	/** 開いているAPFSのパーティション（識別子） */
	const expanded = new SvelteSet<string>();

	function toggle(id: string): void {
		if (expanded.has(id)) expanded.delete(id);
		else expanded.add(id);
	}
</script>

{#if rows.length > 0}
	<section>
		<h3 class="section-title">{m['other.section_title']()}</h3>
		<cds-table size="sm">
			<cds-table-head>
				<cds-table-header-row>
					<cds-table-header-cell>{m['table.identifier']()}</cds-table-header-cell>
					<cds-table-header-cell>{m['table.name']()}</cds-table-header-cell>
					<cds-table-header-cell>{m['table.type']()}</cds-table-header-cell>
					<cds-table-header-cell>{m['table.size']()}</cds-table-header-cell>
					<cds-table-header-cell>{m['table.mount_point']()}</cds-table-header-cell>
				</cds-table-header-row>
			</cds-table-head>
			<cds-table-body>
				{#each rows as partition (partition.id)}
					{@const container = partition.apfs}
					{@const open = expanded.has(partition.id)}
					<cds-table-row onclick={container ? () => toggle(partition.id) : undefined}>
						<cds-table-cell>
							<span class="id mono">
								{#if container}
									<button
										type="button"
										class="toggle"
										aria-expanded={open}
										aria-label={m['other.toggle_container']()}
									>
										<!-- 開閉は行のクリックが受け持つ（ボタンのクリックも行に伝わる） -->
										<Icon icon={open ? icons.chevronDown : icons.chevronRight} />
									</button>
								{:else}
									<span class="pad"></span>
								{/if}
								{partition.id}
							</span>
						</cds-table-cell>
						<cds-table-cell>
							{partition.name ?? EMPTY_VALUE}
							{#if container}
								<span class="muted">· {m['other.volume_count']({ count: container.volumes.length })}</span>
							{/if}
						</cds-table-cell>
						<cds-table-cell><span class="mono">{partition.filesystem ?? EMPTY_VALUE}</span></cds-table-cell>
						<cds-table-cell><span class="mono">{formatBytes(partition.size)}</span></cds-table-cell>
						<cds-table-cell>
							<span class="mono">{partition.mount_point ?? m['table.not_mounted']()}</span>
						</cds-table-cell>
					</cds-table-row>
					{#if container && open}
						<cds-table-row class="child">
							<cds-table-cell><span class="id mono level-1">{container.id}</span></cds-table-cell>
							<cds-table-cell>{m['other.container_label']()}</cds-table-cell>
							<cds-table-cell><span class="mono">{partition.filesystem ?? EMPTY_VALUE}</span></cds-table-cell>
							<cds-table-cell><span class="mono">{formatBytes(partition.size)}</span></cds-table-cell>
							<cds-table-cell><span class="mono">{m['table.not_mounted']()}</span></cds-table-cell>
						</cds-table-row>
						{#each container.volumes as volume (volume.id)}
							<cds-table-row class="child">
								<cds-table-cell><span class="id mono level-2">{volume.id}</span></cds-table-cell>
								<cds-table-cell>{volume.name}</cds-table-cell>
								<cds-table-cell><span class="mono">{partition.filesystem ?? EMPTY_VALUE}</span></cds-table-cell>
								<cds-table-cell>
									<span class="mono">
										{volume.used === null ? EMPTY_VALUE : m['table.volume_used']({ size: formatBytes(volume.used) })}
									</span>
								</cds-table-cell>
								<cds-table-cell>
									<span class="mono">{volume.mount_point ?? m['table.not_mounted']()}</span>
								</cds-table-cell>
							</cds-table-row>
						{/each}
					{/if}
				{/each}
			</cds-table-body>
		</cds-table>
	</section>
{/if}

<style>
	.section-title {
		margin: 0 0 var(--cds-spacing-04);
		font-size: var(--cds-body-compact-01-font-size);
		font-weight: 600;
	}

	.id {
		display: inline-flex;
		align-items: center;
		gap: var(--cds-spacing-02);
	}

	.pad {
		display: inline-block;
		width: 1.5rem;
	}

	.toggle {
		all: unset;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.5rem;
		height: 1.5rem;
		border-radius: var(--esp-radius-sm);
		color: var(--cds-icon-secondary);
		cursor: pointer;
	}

	.toggle:hover {
		background-color: var(--cds-layer-hover-01);
	}

	.toggle:focus-visible {
		outline: var(--cds-spacing-01) solid var(--cds-focus);
	}

	.muted {
		color: var(--cds-text-secondary);
	}

	/* 入れ子の行は、背景と文字の色を一段下げ、識別子の頭を段ごとに下げる */
	.child {
		color: var(--cds-text-secondary);
	}

	.level-1 {
		padding-left: 1.5rem;
	}

	.level-2 {
		padding-left: 2.75rem;
	}
</style>
