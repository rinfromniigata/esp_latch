<script lang="ts">
	// パーティションの帯グラフ。容量の比率で並べ、小さな区画も最小幅で見せる。各区画に、識別子・名前・容量のツールチップ
	// （ホバーとキーボードのフォーカスで出す）。凡例と、最小幅で描いている旨の注記を添える
	import { m } from '#lib/i18n/index.js';
	import type { Device } from '#lib/api/types.js';
	import { formatBytes } from '#lib/format/index.js';
	import { isEsp, partitionColorVar, partitionWeight } from '#lib/devices/model.js';

	let { device }: { device: Device } = $props();

	/** 区画の幅の下限（全体に対する比率） */
	const FLOOR = 0.0001;
	/** 区画の下に識別子の末尾（s1など）を出す最小の比率。ESPは常に出す */
	const LABEL_MIN = 0.04;

	const segments = $derived(
		device.partitions.map((partition, index) => {
			const weight = partitionWeight(partition, device, FLOOR);
			const tip = [partition.id, partition.name, formatBytes(partition.size)].filter(Boolean).join(' · ');
			return {
				partition,
				weight,
				tip,
				suffix: partition.id.startsWith(device.id) ? partition.id.slice(device.id.length) : partition.id,
				showLabel: isEsp(partition) || weight >= LABEL_MIN,
				align: index === 0 ? 'start' : index === device.partitions.length - 1 ? 'end' : 'center'
			};
		})
	);

	const LEGEND = [
		{ color: 'var(--esp-part-esp)', label: () => m['band.legend.esp']() },
		{ color: 'var(--esp-state-mounted)', label: () => m['band.legend.mounted']() },
		{ color: 'var(--esp-part-apfs)', label: () => m['band.legend.apfs']() },
		{ color: 'var(--esp-part-hfs)', label: () => m['band.legend.hfs']() },
		{ color: 'var(--esp-part-other)', label: () => m['band.legend.other']() }
	];
</script>

<div>
	<h3 class="section-title">{m['band.title']()}</h3>
	<div class="band" role="list">
		{#each segments as segment (segment.partition.id)}
			<!-- ツールチップをキーボードのフォーカスでも出すため（スペック6章のアクセシビリティ）、フォーカスを受ける -->
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<div
				class="seg"
				class:esp={isEsp(segment.partition)}
				role="listitem"
				tabindex="0"
				aria-label={segment.tip}
				style:flex="{segment.weight} 1 0"
				style:background-color={partitionColorVar(segment.partition)}
			>
				<span class="tip {segment.align}" aria-hidden="true">{segment.tip}</span>
			</div>
		{/each}
	</div>
	<div class="labels" aria-hidden="true">
		{#each segments as segment (segment.partition.id)}
			<span class="mono" class:esp={isEsp(segment.partition)} style:flex="{segment.weight} 1 0">
				{segment.showLabel ? segment.suffix : ''}
			</span>
		{/each}
	</div>
	<div class="legend">
		{#each LEGEND as item (item.color)}
			<span><i style:background-color={item.color}></i>{item.label()}</span>
		{/each}
		<span>· {m['band.note']()}</span>
	</div>
</div>

<style>
	.section-title {
		margin: 0 0 var(--cds-spacing-04);
		font-size: var(--cds-body-compact-01-font-size);
		font-weight: 600;
	}

	.band,
	.labels {
		display: flex;
		gap: var(--cds-spacing-01);
	}

	.band {
		height: 1.75rem;
	}

	.seg {
		position: relative;
		height: 100%;
		min-width: 0.625rem;
		border-radius: var(--esp-radius-sm);
	}

	.seg.esp {
		min-width: 1.125rem;
	}

	.seg:focus-visible {
		outline: var(--cds-spacing-01) solid var(--cds-focus);
		outline-offset: var(--cds-spacing-01);
	}

	.tip {
		display: none;
		position: absolute;
		bottom: calc(100% + var(--cds-spacing-02));
		z-index: 2;
		padding: var(--cds-spacing-02) var(--cds-spacing-03);
		border-radius: var(--esp-radius-sm);
		background-color: var(--cds-background-inverse);
		color: var(--cds-text-inverse);
		font-size: var(--cds-label-01-font-size);
		white-space: nowrap;
		pointer-events: none;
	}

	.tip.start {
		left: 0;
	}

	.tip.end {
		right: 0;
	}

	.tip.center {
		left: 50%;
		transform: translateX(-50%);
	}

	.seg:hover .tip,
	.seg:focus-visible .tip {
		display: block;
	}

	.labels {
		margin-top: var(--cds-spacing-02);
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
	}

	.labels span {
		min-width: 0.625rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.labels .esp {
		min-width: 1.125rem;
		color: var(--cds-link-primary);
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: var(--cds-spacing-05);
		margin-top: var(--cds-spacing-03);
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
	}

	.legend i {
		display: inline-block;
		width: 0.625rem;
		height: 0.625rem;
		margin-right: var(--cds-spacing-02);
		border-radius: var(--esp-radius-sm);
		vertical-align: baseline;
	}
</style>
