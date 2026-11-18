// Rustのイベント（listen）の型つきの窓口。戻り値の関数で購読をやめる
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { BusyState, DeviceSnapshot, LogEntry, Settings } from './types';

export function onDevicesChanged(handler: (snapshot: DeviceSnapshot) => void): Promise<UnlistenFn> {
	return listen<DeviceSnapshot>('devices-changed', (event) => handler(event.payload));
}

export function onLogAppended(handler: (entry: LogEntry) => void): Promise<UnlistenFn> {
	return listen<LogEntry>('log-appended', (event) => handler(event.payload));
}

export function onLogCleared(handler: () => void): Promise<UnlistenFn> {
	return listen('log-cleared', () => handler());
}

export function onBusy(handler: (state: BusyState) => void): Promise<UnlistenFn> {
	return listen<BusyState>('busy', (event) => handler(event.payload));
}

export function onSettingsChanged(handler: (settings: Settings) => void): Promise<UnlistenFn> {
	return listen<Settings>('settings-changed', (event) => handler(event.payload));
}
