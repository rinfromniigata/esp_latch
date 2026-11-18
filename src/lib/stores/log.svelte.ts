// 操作ログの状態。正本は本体のメモリ内のログで、ここは読み込みとlog-appended・log-clearedの受信だけを持つ
import { clearLog, getLog } from '#lib/api/commands.js';
import { onLogAppended, onLogCleared } from '#lib/api/events.js';
import type { LogEntry } from '#lib/api/types.js';

/** 本体の上限（imple 6章）。同じ件数でそろえる */
export const LOG_LIMIT = 1000;

let entries = $state<readonly LogEntry[]>([]);
let started: Promise<void> | undefined;

function same(a: LogEntry, b: LogEntry): boolean {
	return a.at === b.at && a.command === b.command && a.result === b.result;
}

export const logStore = {
	get entries(): readonly LogEntry[] {
		return entries;
	},
	/** 最後の記録（ステータスバーの「直前のコマンド」） */
	get last(): LogEntry | null {
		return entries.at(-1) ?? null;
	},

	/** 読み込みと購読（1回だけ） */
	start(): Promise<void> {
		started ??= (async () => {
			let loaded = false;
			const early: LogEntry[] = [];
			await onLogAppended((entry) => {
				if (!loaded) early.push(entry);
				else entries = [...entries, entry].slice(-LOG_LIMIT);
			}).catch(() => undefined);
			await onLogCleared(() => (entries = [])).catch(() => undefined);
			try {
				const initial = await getLog();
				// 読み込み中に届いた記録は、取得結果に含まれていなければ足す
				entries = [...initial, ...early.filter((entry) => !initial.some((known) => same(known, entry)))].slice(
					-LOG_LIMIT
				);
			} catch {
				entries = early;
			} finally {
				loaded = true;
			}
		})();
		return started;
	},

	/** 消去を依頼する。空になった結果はlog-clearedで届く */
	async clear(): Promise<void> {
		await clearLog();
	}
};
