// 補助プロセスの状態（認証待ち・管理者権限で実行中）。ステータスバーに出す
import { onPrivilegedStatus } from '#lib/api/events.js';
import type { PrivilegedStatus } from '#lib/api/types.js';

let status = $state<PrivilegedStatus>('none');
let started: Promise<void> | undefined;

export const privilegedStore = {
	get status(): PrivilegedStatus {
		return status;
	},

	/** 購読する（1回だけ）。イベントはPhase 5で本体が出す */
	start(): Promise<void> {
		started ??= onPrivilegedStatus((next) => (status = next)).then(
			() => undefined,
			() => undefined
		);
		return started;
	}
};
