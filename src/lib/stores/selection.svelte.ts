// 選択中のデバイスと、サイドバーの絞り込み
import type { DefaultFilter, Device } from '#lib/api/types.js';
import { filterDevices } from '#lib/devices/model.js';
import { deviceStore } from './devices.svelte';
import { settingsStore } from './settings.svelte';

let selectedId = $state<string | null>(null);
let filter = $state<DefaultFilter>('all');
let initialized = false;

/** 絞り込みを通ったデバイス */
const visible = $derived(filterDevices(deviceStore.devices, filter));
/** 選択中のデバイス。絞り込みで外れたり、取り外されたりしたら、選択なしの扱い */
const selected = $derived<Device | null>(visible.find((device) => device.id === selectedId) ?? null);

export const selectionStore = {
	get filter(): DefaultFilter {
		return filter;
	},
	get visible(): readonly Device[] {
		return visible;
	},
	get selected(): Device | null {
		return selected;
	},
	get selectedId(): string | null {
		return selectedId;
	},

	select(id: string | null): void {
		selectedId = id;
	},

	setFilter(next: DefaultFilter): void {
		filter = next;
	},

	/**
	 * 起動時の絞り込み（設定）を適用し、最初に見えるデバイスを選ぶ。1回だけ行い、
	 * 設定を変えても、いま選んでいる絞り込みは変えない（起動時の絞り込みなので）
	 */
	initialize(): void {
		if (initialized) return;
		initialized = true;
		filter = settingsStore.settings.default_filter;
		selectedId = visible[0]?.id ?? null;
	}
};
