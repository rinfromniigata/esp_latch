<script lang="ts">
	// ESPの容量バー（独自の部品）：空き容量の文字、使用中・ゴミ箱・空きの帯（ツールチップでそれぞれの量）、凡例。
	// 値の色はトークン（使用中は帯グラフのAPFSの色、ゴミ箱は警告の色）。空きが15%を切ったら、空きの文字をエラーの色にする。読めない値は「不明」
	import { m } from '#lib/i18n/index.js';
	import type { EspUsage } from '#lib/api/types.js';
	import { isLowFreeSpace } from '#lib/devices/model.js';
	import { formatBytes } from '#lib/format/index.js';

	let { usage }: { usage: EspUsage } = $props();

	const total = $derived(usage.total ?? 0);
	const known = $derived(usage.total !== null && usage.free !== null && total > 0);
	const trash = $derived(usage.trash ?? 0);
	const used = $derived(known ? Math.max(total - (usage.free ?? 0) - trash, 0) : 0);
	const low = $derived(isLowFreeSpace(usage));
	const percent = (value: number) => `${((value / total) * 100).toFixed(2)}%`;
</script>

{#if known && usage.free !== null && usage.total !== null}
	<span class="mono" class:low>{m['esp.free_of']({ free: formatBytes(usage.free), total: formatBytes(usage.total) })}</span>
	<div
		class="bar"
		aria-hidden="true"
		title={m['esp.usage.summary']({
			used: formatBytes(used),
			trash: formatBytes(trash),
			free: formatBytes(usage.free)
		})}
	>
		<span class="used" style:width={percent(used)}></span>
		<span class="trash" style:width={percent(trash)}></span>
	</div>
	<div class="legend">
		<span><i class="used"></i>{m['esp.usage.used']()}</span>
		<span><i class="trash"></i>{m['esp.usage.trash']()}</span>
		<span><i class="free"></i>{m['esp.usage.free']()}</span>
	</div>
{:else}
	<span class="muted">{m['esp.unknown']()}</span>
{/if}

<style>
	.mono {
		font-family: var(--esp-font-mono);
	}

	.muted {
		color: var(--cds-text-secondary);
	}

	.low {
		color: var(--cds-support-error);
	}

	.bar {
		display: flex;
		max-width: 13.75rem;
		height: 0.375rem;
		margin-top: var(--cds-spacing-02);
		overflow: hidden;
		background-color: var(--cds-layer-selected-01);
		border-radius: var(--esp-radius-sm);
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: var(--cds-spacing-04);
		margin-top: var(--cds-spacing-02);
		font-size: var(--cds-helper-text-01-font-size);
		color: var(--cds-text-secondary);
	}

	.legend i {
		display: inline-block;
		width: 0.5rem;
		height: 0.5rem;
		margin-right: var(--cds-spacing-01);
		border-radius: var(--esp-radius-sm);
	}

	.used {
		background-color: var(--esp-part-apfs);
	}

	.trash {
		background-color: var(--cds-support-warning);
	}

	.legend i.free {
		background-color: var(--cds-layer-selected-01);
		border: 1px solid var(--cds-border-subtle-01);
		box-sizing: border-box;
	}
</style>
