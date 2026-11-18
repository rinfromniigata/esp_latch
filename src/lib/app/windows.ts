// アプリのウィンドウ（メイン・設定・操作ログ）を開く・タイトルを付ける
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { openWindow, type AppWindow } from '#lib/api/commands.js';
import { m } from '#lib/i18n/index.js';
import type { ThemePreference } from '#lib/theme/index.js';

/** ウィンドウのタイトル（現在のロケール） */
export function windowTitle(window: AppWindow): string {
	switch (window) {
		case 'main':
			return m['app.name']();
		case 'settings':
			return m['settings.title']();
		case 'log':
			return m['log.title']();
	}
}

/** ウィンドウを開く（2つ目は作らず前面に出す） */
export function openAppWindow(window: AppWindow): Promise<void> {
	return openWindow(window, windowTitle(window));
}

/** 今のウィンドウのタイトルを、現在のロケールで付け直す */
export async function retitleCurrentWindow(window: AppWindow): Promise<void> {
	if (!isTauri()) return;
	await getCurrentWindow().setTitle(windowTitle(window));
}

/** ウィンドウの枠（タイトルバー）の外観を、アプリの外観の設定にそろえる。systemならOSに従う */
export async function syncWindowTheme(preference: ThemePreference): Promise<void> {
	if (!isTauri()) return;
	await getCurrentWindow().setTheme(preference === 'system' ? null : preference);
}
