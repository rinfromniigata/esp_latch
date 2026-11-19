<script lang="ts">
	// ESPのタイル：識別子・名前、「起動に使用」の印、状態（未マウント・マウント中）、容量・形式・マウント先・ブートローダー・
	// 空き容量（マウント中は容量バーとゴミ箱の量）、操作のボタン（マウント・取り外し・Finderで開く、処理中）。
	// 状態は色だけでなく文字でも示す。ゴミ箱に残りがあれば、警告と「ゴミ箱を空にして取り外す」を主な操作にする（S01k）
	import { m } from '#lib/i18n/index.js';
	import type { Partition } from '#lib/api/types.js';
	import { formatBytes } from '#lib/format/index.js';
	import { EMPTY_VALUE, loaderLabel } from '#lib/devices/labels.js';
	import { hasTrash } from '#lib/devices/model.js';
	import { deviceStore, espStore } from '#lib/stores/index.js';
	import EspUsageBar from './EspUsageBar.svelte';
	import Icon from './Icon.svelte';
	import * as icons from './icons';

	interface Props {
		partition: Partition;
		/** 起動に使ったESPか（OpenCoreの起動パスと一致） */
		boot: boolean;
	}

	let { partition, boot }: Props = $props();

	const mounted = $derived(partition.mount_point !== null);
	/** 本体が操作を実行中（認証待ちを含む） */
	const busy = $derived(deviceStore.isBusy(partition.id));
	/** マウント中のESPの詳細。読み込むまではundefined */
	const detail = $derived(mounted ? espStore.detail(partition.id) : undefined);
	const loader = $derived(
		detail === undefined ? EMPTY_VALUE : detail.loader ? loaderLabel(detail.loader) : m['esp.loader_none']()
	);
	/** ゴミ箱にファイルが残っているマウント中のESP */
	const trashed = $derived(detail !== undefined && hasTrash(detail.usage));
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
		<dt>{m['esp.loader']()}</dt>
		<dd>
			{#if mounted}{loader}{:else}<span class="muted">{m['esp.loader_unknown']()}</span>{/if}
		</dd>
		<dt>{m['esp.free']()}</dt>
		<dd>
			{#if !mounted}
				<span class="muted">{m['esp.free_unknown']()}</span>
			{:else if detail}
				<EspUsageBar usage={detail.usage} />
			{:else}
				{EMPTY_VALUE}
			{/if}
		</dd>
		{#if mounted}
			<dt>{m['esp.trash']()}</dt>
			<dd class="mono">
				{#if detail === undefined}
					{EMPTY_VALUE}
				{:else if detail.usage.trash === null}
					<span class="muted">{m['esp.unknown']()}</span>
				{:else if detail.usage.trash > 0}
					{formatBytes(detail.usage.trash)}
				{:else}
					<span class="muted">{m['esp.trash_none']()}</span>
				{/if}
			</dd>
		{/if}
	</dl>
	{#if trashed && detail && detail.usage.trash !== null}
		<div class="warn" role="status">
			<Icon icon={icons.warning} size="1rem" />
			<span>{m['esp.trash_warning']({ size: formatBytes(detail.usage.trash) })}</span>
		</div>
	{/if}
	<div class="actions">
		{#if busy}
			<cds-button kind="primary" size="md" disabled>
				<cds-inline-loading class="working" status="active">{m['esp.working']()}</cds-inline-loading>
			</cds-button>
		{:else if mounted && trashed}
			<cds-button kind="secondary" size="md" onclick={() => espStore.unmount(partition.id, true)}>
				<Icon icon={icons.eject} />
				{m['esp.empty_trash_unmount']()}
			</cds-button>
			<cds-button kind="ghost" size="md" onclick={() => espStore.emptyTrash(partition.id)}>
				<Icon icon={icons.trash} />
				{m['esp.empty_trash']()}
			</cds-button>
			<cds-button kind="ghost" size="md" onclick={() => espStore.open(partition.id)}>
				<Icon icon={icons.folder} />
				{m['esp.open_finder']()}
			</cds-button>
		{:else if mounted}
			<cds-button kind="secondary" size="md" onclick={() => espStore.unmount(partition.id)}>
				<Icon icon={icons.eject} />
				{m['esp.unmount']()}
			</cds-button>
			<cds-button kind="ghost" size="md" onclick={() => espStore.open(partition.id)}>
				<Icon icon={icons.folder} />
				{m['esp.open_finder']()}
			</cds-button>
		{:else}
			<cds-button kind="primary" size="md" onclick={() => espStore.mount(partition.id)}>
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
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.mono {
		font-family: var(--esp-font-mono);
	}

	.muted {
		color: var(--cds-text-secondary);
		font-family: var(--esp-font-sans);
	}

	/* ゴミ箱の警告：警告の色の左線（モックのinline-warn） */
	.warn {
		display: flex;
		align-items: flex-start;
		gap: var(--cds-spacing-03);
		padding: var(--cds-spacing-03) var(--cds-spacing-04);
		font-size: var(--cds-label-01-font-size);
		background-color: var(--cds-layer-02);
		border-left: 3px solid var(--cds-support-warning);
	}

	.warn :global(svg) {
		flex: none;
		fill: var(--cds-support-warning);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--cds-spacing-03);
	}

	/* 処理中のボタンの中のローディング。ボタンの文字色にそろえる */
	.working {
		min-height: 0;
		color: inherit;
	}
</style>
