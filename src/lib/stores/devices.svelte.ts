// デバイスの一覧の状態。取得は本体が行い、ここは受け取り・更新の依頼・処理中の対象の記録だけを持つ
import { SvelteSet } from 'svelte/reactivity';
import { listDevices, refresh } from '#lib/api/commands.js';
import { onBusy, onDevicesChanged } from '#lib/api/events.js';
import { toAppError } from '#lib/api/errors.js';
import type { AppError, BusyState, Device, DeviceSnapshot } from '#lib/api/types.js';

let snapshot = $state<DeviceSnapshot | null>(null);
/** 起動直後の取得中、または更新ボタンの再取得中（スケルトンを出す） */
let loading = $state(true);
let error = $state<AppError | null>(null);
/** 操作を実行中の対象（BSD名）。ボタンを処理中にするために使う */
const busyTargets = new SvelteSet<string>();
let started: Promise<void> | undefined;

function applyBusy(state: BusyState): void {
	if (state.active) busyTargets.add(state.target);
	else busyTargets.delete(state.target);
}

export const deviceStore = {
	get snapshot(): DeviceSnapshot | null {
		return snapshot;
	},
	get devices(): readonly Device[] {
		return snapshot?.devices ?? [];
	},
	get loading(): boolean {
		return loading;
	},
	get error(): AppError | null {
		return error;
	},

	/** 対象（`disk2`・`disk2s1`）への操作を実行中か */
	isBusy(target: string): boolean {
		return busyTargets.has(target);
	},

	/** 一覧を読み込み、変化を購読する（1回だけ）。最初の一覧が届くか、失敗するまで待つ */
	start(): Promise<void> {
		started ??= (async () => {
			await onDevicesChanged((next) => {
				snapshot = next;
				error = null;
				loading = false;
			}).catch(() => undefined);
			await onBusy(applyBusy).catch(() => undefined);
			try {
				snapshot = await listDevices();
				error = null;
			} catch (cause) {
				error = toAppError(cause);
			} finally {
				loading = false;
			}
		})();
		return started;
	},

	/** 再取得する（更新ボタン）。結果は本体のdevices-changedで届く */
	async refresh(): Promise<void> {
		loading = true;
		try {
			await refresh();
		} catch (cause) {
			error = toAppError(cause);
		} finally {
			loading = false;
		}
	}
};
