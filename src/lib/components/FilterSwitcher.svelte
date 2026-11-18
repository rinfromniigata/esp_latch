<script lang="ts">
	// 絞り込み（全て・ESPあり・物理）。Carbonのコンテンツスイッチャー
	import { filterLabel } from '#lib/devices/labels.js';
	import type { DefaultFilter } from '#lib/api/types.js';

	interface Props {
		value: DefaultFilter;
		onchange: (value: DefaultFilter) => void;
	}

	let { value, onchange }: Props = $props();

	const FILTERS: DefaultFilter[] = ['all', 'esp', 'physical'];

	function handleSelected(event: Event): void {
		const selected = (event as CustomEvent<{ item: { value: string } }>).detail.item.value;
		if (FILTERS.includes(selected as DefaultFilter)) onchange(selected as DefaultFilter);
	}
</script>

<cds-content-switcher size="sm" {value} oncds-content-switcher-selected={handleSelected}>
	{#each FILTERS as filter (filter)}
		<cds-content-switcher-item value={filter}>{filterLabel(filter)}</cds-content-switcher-item>
	{/each}
</cds-content-switcher>
