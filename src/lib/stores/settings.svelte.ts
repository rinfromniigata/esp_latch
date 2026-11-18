// 設定の状態。本体の`SettingsStore`が正本で、ここは読み込み・変更の依頼・settings-changedの受信だけを持つ
import { getSettings, updateSettings } from '#lib/api/commands.js';
import { onSettingsChanged } from '#lib/api/events.js';
import type { Settings, SettingsPatch } from '#lib/api/types.js';

/** 本体が読めないとき（Tauriの外など）に使う既定値。Rustの既定値（imple 7章）と同じ */
export const DEFAULT_SETTINGS: Settings = {
	language: 'system',
	theme: 'system',
	default_filter: 'all',
	open_finder_after_mount: true,
	auto_refresh: true,
	confirm_eject: true,
	auto_empty_trash_on_unmount: false,
	show_command: true
};

let current = $state<Settings>({ ...DEFAULT_SETTINGS });
let started: Promise<Settings> | undefined;

export const settingsStore = {
	get settings(): Settings {
		return current;
	},

	/** 設定を読み込み、変更を購読する（1回だけ。すべてのウィンドウが呼ぶ）。読み込み後の設定を返す */
	start(): Promise<Settings> {
		started ??= (async () => {
			// 先に購読して、読み込み中に届いた変更を取りこぼさない
			await onSettingsChanged((settings) => (current = settings)).catch(() => undefined);
			try {
				current = await getSettings();
			} catch {
				// 読めなければ既定値のまま使う
			}
			return current;
		})();
		return started;
	},

	/** 指定した項目だけを変えて保存する。結果は本体からのsettings-changedでも届く */
	async update(patch: SettingsPatch): Promise<void> {
		current = await updateSettings(patch);
	}
};
