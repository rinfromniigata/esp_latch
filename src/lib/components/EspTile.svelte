<script lang="ts">
	// ESPのタイル（表示）：識別子・名前、「起動に使用」の印、状態（未マウント・マウント中）、容量・形式・マウント先、操作のボタン。
	// 状態は色だけでなく文字でも示す。ボタンの操作（マウント・取り外し・Finderで開く）は、Phase 5で接続する（それまでは無効）
	import { m } from '#lib/i18n/index.js';
	import type { Partition } from '#lib/api/types.js';
	import { formatBytes } from '#lib/format/index.js';
	import { EMPTY_VALUE } from '#lib/devices/labels.js';
	import Icon from './Icon.svelte';
	import * as icons from './icons';

	interface Props {
		partition: Partition;
		/** 起動に使ったESPか（OpenCoreの起動パスと一致） */
		boot: boolean;
	}

	let { partition, boot }: Props = $props();

	const mounted = $derived(partition.mount_point !== null);
</script>

<article class="tile" class:mounted>
	<div class="row">
		<div>
			<div class="id">{partition.id}</div>
			{#if partition.name}<div class="name">{partition.name}</div>{/if}
		</div>
		<div class="tags">
			{#if boot}
				<cds-tag size="sm" type="gray">
					<span slot="icon" class="boot-icon"><Icon icon={icons.boot} size="0.75rem" /></span>
					{m['esp.boot_badge']()}
				</cds-tag>
			{/if}
			{#if mounted}
				<cds-tag class="mounted-tag" size="sm" type="green">{m['esp.state.mounted']()}</cds-tag>
			{:else}
				<cds-tag size="sm" type="gray">{m['esp.state.unmounted']()}</cds-tag>
			{/if}
		</div>
	</div>
	<dl class="kv">
		<dt>{m['esp.size']()}</dt>
		<dd class="mono">{formatBytes(partition.size)}</dd>
		<dt>{m['esp.format']()}</dt>
		<dd class="mono">{partition.filesystem ?? EMPTY_VALUE}</dd>
		<dt>{m['esp.mount_point']()}</dt>
		<dd class="mono">{partition.mount_point ?? m['table.not_mounted']()}</dd>
	</dl>
	<div class="actions">
		{#if mounted}
			<cds-button kind="secondary" size="md" disabled>
				<Icon icon={icons.eject} />
				{m['esp.unmount']()}
			</cds-button>
			<cds-button kind="ghost" size="md" disabled>
				<Icon icon={icons.folder} />
				{m['esp.open_finder']()}
			</cds-button>
		{:else}
			<cds-button kind="primary" size="md" disabled>
				<Icon icon={icons.mount} />
				{m['esp.mount']()}
			</cds-button>
		{/if}
	</div>
</article>

<style>
	.tile {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-04);
		padding: var(--cds-spacing-05);
		background-color: var(--cds-layer-01);
		border: 1px solid var(--cds-border-subtle-01);
		border-radius: var(--esp-radius-md);
	}

	.tile.mounted {
		border-color: var(--esp-state-mounted);
	}

	.row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--cds-spacing-03);
	}

	.id {
		font-family: var(--esp-font-mono);
		font-size: var(--cds-heading-compact-02-font-size);
		font-weight: 500;
	}

	.name {
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--cds-spacing-02);
	}

	/* マウント中は、ブランドの状態の色（緑のタグの色を差し替える） */
	.mounted-tag {
		--cds-tag-background-green: var(--esp-state-mounted-bg);
		--cds-tag-color-green: var(--esp-state-mounted);
	}

	.boot-icon {
		display: inline-flex;
	}

	.kv {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: var(--cds-spacing-01) var(--cds-spacing-04);
		margin: 0;
		font-size: var(--cds-label-01-font-size);
	}

	.kv dt {
		color: var(--cds-text-secondary);
	}

	.kv dd {
		margin: 0;
		overflow-wrap: anywhere;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--cds-spacing-03);
	}
</style>
