<script lang="ts">
	// デバイスのタグ：種別・ESPの数（なければ「ESPなし」）・マウント中のESPの数。状態は色だけでなく文字でも示す
	import { m } from '#lib/i18n/index.js';
	import type { Device } from '#lib/api/types.js';
	import { kindLabel } from '#lib/devices/labels.js';
	import { espCount, mountedEspCount } from '#lib/devices/model.js';

	let { device }: { device: Device } = $props();

	const esp = $derived(espCount(device));
	const mounted = $derived(mountedEspCount(device));
</script>

<div class="tags">
	<cds-tag size="sm" type="gray">{kindLabel(device.kind)}</cds-tag>
	{#if esp > 0}
		<cds-tag size="sm" type="blue">{m['device.esp_count']({ count: esp })}</cds-tag>
	{:else}
		<cds-tag size="sm" type="gray">{m['device.esp_none']()}</cds-tag>
	{/if}
	{#if mounted > 0}
		<cds-tag class="mounted" size="sm" type="green">{m['device.mounted_count']({ count: mounted })}</cds-tag>
	{/if}
</div>

<style>
	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: var(--cds-spacing-02);
	}

	/* マウント中は、ブランドの状態の色（緑のタグの色を差し替える） */
	.mounted {
		--cds-tag-background-green: var(--esp-state-mounted-bg);
		--cds-tag-color-green: var(--esp-state-mounted);
	}
</style>
