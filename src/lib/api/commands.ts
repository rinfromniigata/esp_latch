// Rustのコマンド（invoke）の型つきの窓口。UIからはここを通してだけ呼ぶ
import { invoke } from '@tauri-apps/api/core';

/** OSの優先言語（優先順のBCP 47の言語タグ） */
export function systemLocales(): Promise<string[]> {
	return invoke<string[]>('system_locales');
}

export type AppWindow = 'main' | 'settings' | 'log';

/** ウィンドウを開く。開いていれば前面に出す。titleは現在のロケールのタイトル */
export function openWindow(window: AppWindow, title: string): Promise<void> {
	return invoke('open_window', { window, title });
}
