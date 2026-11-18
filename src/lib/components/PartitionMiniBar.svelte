<script lang="ts">
	// サイドバーの小さな帯。パーティションの配置を、容量の比率で細く示す（読み上げには出さない）
	import type { Device } from '#lib/api/types.js';
	import { partitionColorVar, partitionWeight } from '#lib/devices/model.js';

	let { device }: { device: Device } = $props();

	/** 区画の幅の下限（全体に対する比率） */
	const FLOOR = 0.02;
</script>

<div class="minibar" aria-hidden="true">
	{#each device.partitions as partition (partition.id)}
		<span
			style:flex="{partitionWeight(partition, device, FLOOR)} 1 0"
			style:background-color={partitionColorVar(partition)}
		></span>
	{/each}
</div>

<style>
	.minibar {
		display: flex;
		gap: 1px;
		height: var(--cds-spacing-02);
		margin-top: var(--cds-spacing-03);
	}

	.minibar span {
		display: block;
		height: 100%;
		min-width: 0;
	}
</style>
