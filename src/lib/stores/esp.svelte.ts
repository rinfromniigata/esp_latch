// マウント中のESPの詳細（ブートローダー・容量）と、ESPの操作（マウント・取り外し・Finderで開く）。
// 操作の処理中は、本体のbusyイベント（deviceStore.isBusy）で示す。結果とエラーは通知で知らせる
import { SvelteMap } from 'svelte/reactivity';
import { ejectDevice, emptyEspTrash, inspectEsp, mountEsp, openEsp, unmountEsp } from '#lib/api/commands.js';
import { describeError, toAppError } from '#lib/api/errors.js';
import type { AppError, EspDetail, Operation, Partition } from '#lib/api/types.js';
import { openAppWindow } from '#lib/app/windows.js';
import { formatBytes } from '#lib/format/index.js';
import { m } from '#lib/i18n/index.js';
import { toastStore, type ToastActionId } from './toasts.svelte';

/** 成功の通知を出しておく時間 */
export const SUCCESS_TOAST_MS = 5000;

const details = new SvelteMap<string, EspDetail>();

/** エラーの通知に付けるボタン。認証のキャンセルは再試行、コマンドの失敗は再試行とログ */
export function errorActions(error: AppError): ToastActionId[] {
	switch (error.kind) {
		case 'auth_cancelled':
			return ['retry'];
		case 'command_failed':
			return ['retry', 'show_log'];
		default:
			return [];
	}
}

function notifyError(cause: unknown, op: Operation, retry: () => Promise<void>): void {
	const error = toAppError(cause);
	const id = toastStore.pushError(describeError(error, op), {
		actions: errorActions(error),
		onAction: (action) => {
			toastStore.dismiss(id);
			if (action === 'retry') void retry();
			else void openAppWindow('log');
		}
	});
}

export const espStore = {
	/** マウント中のESPの詳細。まだ読んでいない・読めなかったときはundefined */
	detail(part: string): EspDetail | undefined {
		return details.get(part);
	},

	/** 表示中のESPの詳細を読み直す。マウント中のものは読み、取り外されたものは消す */
	async sync(partitions: readonly Partition[]): Promise<void> {
		await Promise.all(
			partitions.map(async (partition) => {
				if (partition.kind !== 'esp') return;
				if (partition.mount_point === null) {
					details.delete(partition.id);
					return;
				}
				try {
					details.set(partition.id, await inspectEsp(partition.id));
				} catch {
					details.delete(partition.id);
				}
			})
		);
	},

	/** マウントする。成功したら詳細を持ち、通知を出す */
	async mount(part: string): Promise<void> {
		try {
			const detail = await mountEsp(part);
			details.set(part, detail);
			toastStore.push({
				kind: 'ok',
				title: m['toast.mount_success.title'](),
				body: m['toast.mount_success.body']({ id: part, path: detail.mount_point }),
				timeoutMs: SUCCESS_TOAST_MS
			});
		} catch (cause) {
			notifyError(cause, 'mount_esp', () => this.mount(part));
		}
	},

	/** 取り外す。emptyTrashなら、先にそのESPのゴミ箱を空にする（失敗したら取り外さない） */
	async unmount(part: string, emptyTrash = false): Promise<void> {
		try {
			await unmountEsp(part, emptyTrash);
			details.delete(part);
		} catch (cause) {
			notifyError(cause, 'unmount_esp', () => this.unmount(part, emptyTrash));
		}
	},

	/** そのESPのゴミ箱を空にし、解放した容量を通知する。詳細（空き容量・ゴミ箱の量）は、本体の一覧の更新で読み直される */
	async emptyTrash(part: string): Promise<void> {
		try {
			const freed = await emptyEspTrash(part);
			toastStore.push({
				kind: 'ok',
				title: m['toast.trash_emptied.title'](),
				body: m['toast.trash_emptied.body']({ id: part, size: formatBytes(freed) }),
				timeoutMs: SUCCESS_TOAST_MS
			});
		} catch (cause) {
			notifyError(cause, 'empty_esp_trash', () => this.emptyTrash(part));
		}
	},

	/** デバイスごと取り出す。成功すると一覧からデバイスが消える（本体の一覧の更新）。確認は呼び出し側が済ませる */
	async eject(disk: string): Promise<void> {
		try {
			await ejectDevice(disk);
			for (const part of [...details.keys()]) if (part.startsWith(`${disk}s`)) details.delete(part);
		} catch (cause) {
			notifyError(cause, 'eject_device', () => this.eject(disk));
		}
	},

	async open(part: string): Promise<void> {
		try {
			await openEsp(part);
		} catch (cause) {
			notifyError(cause, 'open_esp', () => this.open(part));
		}
	}
};
