// 通知（トースト）の状態。表示はToastStack（Phase 5）が受け持つ
import type { ErrorMessage } from '#lib/api/errors.js';

export type ToastKind = 'ok' | 'error';

/** 通知のボタン。どの動作かは、押されたときの呼び出し側が決める */
export type ToastActionId = 'retry' | 'show_log';

export interface Toast {
	id: number;
	kind: ToastKind;
	title: string;
	body?: string;
	/** 実行したコマンドの原文 */
	command?: string;
	/** 標準エラー出力の原文 */
	stderr?: string;
	actions: ToastActionId[];
	/** 押されたボタンの動作 */
	onAction?: (action: ToastActionId) => void;
}

export type ToastInput = Omit<Toast, 'id' | 'actions'> & { actions?: ToastActionId[]; timeoutMs?: number };

let items = $state<readonly Toast[]>([]);
let nextId = 1;

function dismiss(id: number): void {
	items = items.filter((toast) => toast.id !== id);
}

export const toastStore = {
	get items(): readonly Toast[] {
		return items;
	},

	/** 通知を出す。timeoutMsがあれば、その時間で消す。戻り値は通知のID */
	push({ timeoutMs, actions = [], ...input }: ToastInput): number {
		const id = nextId++;
		items = [...items, { ...input, id, actions }];
		if (timeoutMs !== undefined) setTimeout(() => dismiss(id), timeoutMs);
		return id;
	},

	/** エラーの文言（describeError）から、エラーの通知を出す */
	pushError(message: ErrorMessage, options: Pick<ToastInput, 'actions' | 'onAction'> = {}): number {
		return this.push({ kind: 'error', ...message, ...options });
	},

	dismiss,

	clear(): void {
		items = [];
	}
};
