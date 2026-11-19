// Rustのコマンド（invoke）の型つきの窓口。UIからはここを通してだけ呼ぶ
import { invoke } from '@tauri-apps/api/core';
import type { DeviceSnapshot, EspDetail, LogEntry, Settings, SettingsPatch } from './types';

/** OSの優先言語（優先順のBCP 47の言語タグ） */
export function systemLocales(): Promise<string[]> {
	return invoke<string[]>('system_locales');
}

export type AppWindow = 'main' | 'settings' | 'log';

/** ウィンドウを開く。開いていれば前面に出す。titleは現在のロケールのタイトル */
export function openWindow(window: AppWindow, title: string): Promise<void> {
	return invoke('open_window', { window, title });
}

/** デバイスの一覧。取得済みならそれを返す */
export function listDevices(): Promise<DeviceSnapshot> {
	return invoke<DeviceSnapshot>('list_devices');
}

/** 再取得して devices-changed を出す */
export function refresh(): Promise<void> {
	return invoke('refresh');
}

/** マウント中のESPの詳細（ブートローダー、容量） */
export function inspectEsp(part: string): Promise<EspDetail> {
	return invoke<EspDetail>('inspect_esp', { part });
}

/** ESPをマウントする。初回は管理者の認証が出る。設定に従ってFinderで開く */
export function mountEsp(part: string): Promise<EspDetail> {
	return invoke<EspDetail>('mount_esp', { part });
}

/** マウント中のESPを取り外す。emptyTrashなら、先にそのESPのゴミ箱を空にする（設定でオンなら、falseでも空にする） */
export function unmountEsp(part: string, emptyTrash = false): Promise<void> {
	return invoke('unmount_esp', { part, emptyTrash });
}

/** マウント中のESPのゴミ箱を空にする。解放したバイト数を返す */
export function emptyEspTrash(part: string): Promise<number> {
	return invoke<number>('empty_esp_trash', { part });
}

/** デバイスごと取り出す（マウント中のボリュームもすべて取り外される） */
export function ejectDevice(disk: string): Promise<void> {
	return invoke('eject_device', { disk });
}

/** マウント中のESPをFinderで開く */
export function openEsp(part: string): Promise<void> {
	return invoke('open_esp', { part });
}

export function getLog(): Promise<LogEntry[]> {
	return invoke<LogEntry[]>('get_log');
}

/** 操作ログを消し、log-cleared を出す */
export function clearLog(): Promise<void> {
	return invoke('clear_log');
}

export function getSettings(): Promise<Settings> {
	return invoke<Settings>('get_settings');
}

/** 指定した項目だけを変えて保存する。変更があれば settings-changed が全ウィンドウに届く */
export function updateSettings(patch: SettingsPatch): Promise<Settings> {
	return invoke<Settings>('update_settings', { patch });
}

export interface AppInfo {
	version: string;
	/** リポジトリのURL */
	repository: string;
}

/** アプリのバージョンとリポジトリ（表示だけ。保存しない） */
export function appInfo(): Promise<AppInfo> {
	return invoke<AppInfo>('app_info');
}

/** リポジトリのページを既定のブラウザで開く */
export function openRepository(): Promise<void> {
	return invoke('open_repository');
}
