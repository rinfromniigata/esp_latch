<script lang="ts">
	// 詳細の見出しと情報：名前（ほかN個・ベンダー名）、タグ、識別子・接続・方式・容量、ディスクイメージのパスと保存先
	import { m } from '#lib/i18n/index.js';
	import type { Device } from '#lib/api/types.js';
	import { displayNames } from '#lib/devices/model.js';
	import { schemeLabel } from '#lib/devices/labels.js';
	import { formatBytes } from '#lib/format/index.js';
	import { deviceStore, espStore, settingsStore } from '#lib/stores/index.js';
	import EjectDialog from './EjectDialog.svelte';
	import DeviceTags from './DeviceTags.svelte';
	import Icon from './Icon.svelte';
	import * as icons from './icons';

	let { device }: { device: Device } = $props();

	const names = $derived(displayNames(device));
	const icon = $derived(
		device.kind === 'internal' ? icons.internalDisk : device.kind === 'external' ? icons.externalDisk : icons.diskImage
	);
	const scheme = $derived(schemeLabel(device.scheme));

	/** このデバイスか、その区画への操作を実行中（取り出しは同じディスクの操作が終わるまで待つ） */
	const busy = $derived(deviceStore.isBusy(device.id) || device.partitions.some((part) => deviceStore.isBusy(part.id)));
	let confirming = $state(false);

	// 選んだデバイスが変わったら（切断を含む）、確認を閉じる
	$effect(() => {
		void device.id;
		confirming = false;
	});

	/** 設定の`confirm_eject`がオンなら確認してから、オフならすぐに取り出す */
	function requestEject(): void {
		if (settingsStore.settings.confirm_eject) confirming = true;
		else void espStore.eject(device.id);
	}

	function confirmEject(): void {
		confirming = false;
		void espStore.eject(device.id);
	}
</script>

<div class="head">
	<Icon {icon} size="2.5rem" class="kind" />
	<div class="grow">
		<h1 title={names.main}>
			{names.main}
			{#if names.more > 0}<span class="more">{m['device.more_labels']({ count: names.more })}</span>{/if}
		</h1>
		{#if names.vendor}<div class="vendor">{names.vendor}</div>{/if}
		<div class="tags"><DeviceTags {device} /></div>
		<dl class="meta">
			<div><dt>{m['detail.identifier']()}</dt><dd class="mono">{device.id}</dd></div>
			{#if device.connection}<div><dt>{m['detail.connection']()}</dt><dd>{device.connection}</dd></div>{/if}
			{#if scheme}<div><dt>{m['detail.scheme']()}</dt><dd>{scheme}</dd></div>{/if}
			<div><dt>{m['detail.capacity']()}</dt><dd class="mono">{formatBytes(device.size)}</dd></div>
			{#if device.image?.stored_on}
				<div>
					<dt>{m['detail.stored_on']()}</dt>
					<dd class="mono">{m['device.on_store']({ store: device.image.stored_on })}</dd>
				</div>
			{/if}
		</dl>
		{#if device.image}
			<!-- 長いパスは、先頭ではなく末尾（ファイル名）が見えるように省略する -->
			<div class="path" title={device.image.path}>&lrm;{device.image.path}</div>
		{/if}
	</div>
	<cds-button kind="tertiary" size="md" disabled={busy} onclick={requestEject}>
		<Icon icon={icons.eject} />
		{m['detail.eject_device']()}
	</cds-button>
</div>
<EjectDialog {device} open={confirming} onconfirm={confirmEject} onclose={() => (confirming = false)} />

<style>
	.head {
		display: flex;
		align-items: flex-start;
		gap: var(--cds-spacing-05);
	}

	.head :global(.kind) {
		color: var(--cds-icon-secondary);
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	h1 {
		margin: 0;
		font-size: var(--cds-heading-03-font-size);
		font-weight: 500;
		overflow-wrap: anywhere;
	}

	.more {
		font-size: var(--cds-label-01-font-size);
		font-weight: 400;
		color: var(--cds-text-secondary);
	}

	.vendor {
		margin-top: var(--cds-spacing-01);
		font-size: var(--cds-body-compact-01-font-size);
		color: var(--cds-text-secondary);
	}

	.tags {
		margin-top: var(--cds-spacing-03);
	}

	.meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--cds-spacing-01) var(--cds-spacing-06);
		margin: var(--cds-spacing-03) 0 0;
		font-size: var(--cds-label-01-font-size);
	}

	.meta dt {
		display: inline;
		margin-right: var(--cds-spacing-03);
		color: var(--cds-text-secondary);
	}

	.meta dd {
		display: inline;
		margin: 0;
	}

	.path {
		margin-top: var(--cds-spacing-02);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		direction: rtl;
		text-align: left;
		font-family: var(--esp-font-mono);
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
	}
</style>
