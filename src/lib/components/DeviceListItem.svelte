<script lang="ts">
	// サイドバーのデバイスの一覧項目：ラベル名・ほかN個・ベンダー名・識別子・タグ・小さな帯
	import { m } from '#lib/i18n/index.js';
	import type { Device } from '#lib/api/types.js';
	import { displayNames } from '#lib/devices/model.js';
	import { formatBytes } from '#lib/format/index.js';
	import DeviceTags from './DeviceTags.svelte';
	import Icon from './Icon.svelte';
	import PartitionMiniBar from './PartitionMiniBar.svelte';
	import * as icons from './icons';

	interface Props {
		device: Device;
		selected: boolean;
		onselect: (id: string) => void;
	}

	let { device, selected, onselect }: Props = $props();

	const names = $derived(displayNames(device));
	const icon = $derived(
		device.kind === 'internal' ? icons.internalDisk : device.kind === 'external' ? icons.externalDisk : icons.diskImage
	);
	/** 省略された長い名前を、ホバーで全文（名前とベンダー名）で見せる */
	const fullName = $derived(names.vendor ? `${names.main} · ${names.vendor}` : names.main);
</script>

<button type="button" class="item" class:selected aria-current={selected ? 'true' : undefined} title={fullName} onclick={() => onselect(device.id)}>
	<Icon {icon} size="1.5rem" class="kind" />
	<span class="text">
		<span class="name">
			<span class="main">{names.main}</span>
			{#if names.more > 0}<span class="more">{m['device.more_labels']({ count: names.more })}</span>{/if}
		</span>
		{#if names.vendor}<span class="line vendor">{names.vendor}</span>{/if}
		<span class="line">
			<span class="mono">{device.id}</span>
			{#if device.image?.stored_on}
				<span>({m['device.on_store']({ store: device.image.stored_on })})</span>
			{/if}
			<span>·</span>
			<span class="mono">{formatBytes(device.size)}</span>
		</span>
		<DeviceTags {device} />
		<PartitionMiniBar {device} />
	</span>
</button>

<style>
	.item {
		all: unset;
		box-sizing: border-box;
		display: flex;
		gap: var(--cds-spacing-04);
		width: 100%;
		padding: var(--cds-spacing-04) var(--cds-spacing-05);
		border-top: 1px solid var(--cds-border-subtle-01);
		cursor: pointer;
		position: relative;
	}

	.item:hover {
		background-color: var(--cds-layer-hover-01);
	}

	.item.selected {
		background-color: var(--cds-layer-selected-01);
	}

	/* 選択中は、左端の線でも示す（色だけに頼らない） */
	.item.selected::before {
		content: '';
		position: absolute;
		inset: 0 auto 0 0;
		width: var(--cds-spacing-02);
		background-color: var(--cds-interactive);
	}

	.item:focus-visible {
		outline: var(--cds-spacing-01) solid var(--cds-focus);
		outline-offset: calc(-1 * var(--cds-spacing-01));
	}

	.item :global(.kind) {
		color: var(--cds-icon-secondary);
		margin-top: var(--cds-spacing-01);
	}

	.text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-01);
	}

	.name {
		display: flex;
		align-items: baseline;
		gap: var(--cds-spacing-02);
		min-width: 0;
		font-size: var(--cds-body-compact-01-font-size);
		font-weight: 500;
	}

	.main {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.more {
		flex: none;
		font-size: var(--cds-label-01-font-size);
		font-weight: 400;
		color: var(--cds-text-secondary);
	}

	.line {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
	}

	.vendor {
		font-size: var(--cds-label-01-font-size);
	}

	.text > :global(.tags) {
		margin-top: var(--cds-spacing-02);
	}
</style>
