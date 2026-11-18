// Rustのコマンド（invoke）の型つきの窓口。UIからはここを通してだけ呼ぶ
import { invoke } from '@tauri-apps/api/core';
import type { DeviceSnapshot, LogEntry, Settings, SettingsPatch } from './types';

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
