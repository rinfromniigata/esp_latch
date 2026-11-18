import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Device, DeviceSnapshot, LogEntry, Settings } from '#lib/api/types.js';

// 本体（Tauri）の代わりに、窓口とイベントを偽物にして、ストアの動きを確かめる
const handlers = vi.hoisted(() => new Map<string, (payload: never) => void>());
const api = vi.hoisted(() => ({
	listDevices: vi.fn(),
	refresh: vi.fn(),
	getLog: vi.fn(),
	clearLog: vi.fn(),
	getSettings: vi.fn(),
	updateSettings: vi.fn()
}));

vi.mock('#lib/api/commands.js', () => api);
vi.mock('#lib/api/events.js', () => {
	const on = (name: string) => async (handler: (payload: never) => void) => {
		handlers.set(name, handler);
		return () => handlers.delete(name);
	};
	return {
		onDevicesChanged: on('devices-changed'),
		onLogAppended: on('log-appended'),
		onLogCleared: on('log-cleared'),
		onBusy: on('busy'),
		onSettingsChanged: on('settings-changed'),
		onPrivilegedStatus: on('privileged-status')
	};
});

const emit = (name: string, payload?: unknown) => handlers.get(name)?.(payload as never);

function device(id: string, kind: Device['kind'], esp: boolean): Device {
	return {
		id,
		kind,
		labels: [],
		extra_label_count: 0,
		vendor: id,
		image: null,
		connection: null,
		scheme: 'gpt',
		size: 1,
		partitions: esp
			? [
					{
						id: `${id}s1`,
						kind: 'esp',
						type_name: 'EFI',
						filesystem: 'FAT32',
						name: 'EFI',
						offset: null,
						size: 1,
						part_uuid: null,
						mount_point: null,
						apfs: null
					}
				]
			: []
	};
}

const snapshot = (...devices: Device[]): DeviceSnapshot => ({ devices, boot_part_uuid: null, taken_at: 0 });
const entry = (at: number, command = 'diskutil list -plist'): LogEntry => ({ at, command, result: 'ok' });

// ストアはモジュールの状態を持つため、テストごとに読み込み直す
async function load() {
	vi.resetModules();
	handlers.clear();
	return import('./index');
}

beforeEach(() => {
	for (const fn of Object.values(api)) fn.mockReset();
});

describe('deviceStore', () => {
	it('起動時に一覧を読み込み、devices-changedで更新し、更新の間はloadingになる', async () => {
		api.listDevices.mockResolvedValue(snapshot(device('disk0', 'internal', true)));
		const { deviceStore } = await load();
		expect(deviceStore.loading).toBe(true);
		await deviceStore.start();
		expect(deviceStore.loading).toBe(false);
		expect(deviceStore.devices.map((d) => d.id)).toEqual(['disk0']);

		emit('devices-changed', snapshot(device('disk0', 'internal', true), device('disk2', 'external', false)));
		expect(deviceStore.devices.map((d) => d.id)).toEqual(['disk0', 'disk2']);

		let finish = () => {};
		api.refresh.mockReturnValue(new Promise<void>((resolve) => (finish = resolve)));
		const refreshing = deviceStore.refresh();
		expect(deviceStore.loading).toBe(true);
		finish();
		await refreshing;
		expect(deviceStore.loading).toBe(false);
	});

	it('取得に失敗したらエラーを持ち、一覧は空のまま', async () => {
		api.listDevices.mockRejectedValue({ kind: 'command_failed', command: 'diskutil list -plist', stderr: 'x' });
		const { deviceStore } = await load();
		await deviceStore.start();
		expect(deviceStore.error).toMatchObject({ kind: 'command_failed' });
		expect(deviceStore.devices).toEqual([]);
		expect(deviceStore.loading).toBe(false);
	});

	it('busyイベントで、対象を処理中にして戻す', async () => {
		api.listDevices.mockResolvedValue(snapshot());
		const { deviceStore } = await load();
		await deviceStore.start();
		emit('busy', { target: 'disk4s1', op: 'mount_esp', active: true });
		expect(deviceStore.isBusy('disk4s1')).toBe(true);
		emit('busy', { target: 'disk4s1', op: 'mount_esp', active: false });
		expect(deviceStore.isBusy('disk4s1')).toBe(false);
	});
});

describe('selectionStore', () => {
	const settings = (default_filter: Settings['default_filter']) =>
		({ default_filter, language: 'system', theme: 'system' }) as Settings;

	it('起動時の絞り込みを適用し、最初に見えるデバイスを選ぶ', async () => {
		api.getSettings.mockResolvedValue(settings('esp'));
		api.listDevices.mockResolvedValue(
			snapshot(device('disk8', 'external', false), device('disk2', 'external', true))
		);
		const { deviceStore, settingsStore, selectionStore } = await load();
		await Promise.all([settingsStore.start(), deviceStore.start()]);
		selectionStore.initialize();
		expect(selectionStore.filter).toBe('esp');
		expect(selectionStore.selected?.id).toBe('disk2');
	});

	it('絞り込みで外れたデバイスは選択なしになり、戻すと選択が戻る。取り外されたら選択なし', async () => {
		api.getSettings.mockResolvedValue(settings('all'));
		api.listDevices.mockResolvedValue(
			snapshot(device('disk2', 'external', true), device('disk6', 'image', true), device('disk8', 'external', false))
		);
		const { deviceStore, settingsStore, selectionStore } = await load();
		await Promise.all([settingsStore.start(), deviceStore.start()]);
		selectionStore.initialize();
		selectionStore.select('disk6');
		selectionStore.setFilter('physical');
		expect(selectionStore.selected).toBeNull();
		selectionStore.setFilter('all');
		expect(selectionStore.selected?.id).toBe('disk6');

		emit('devices-changed', snapshot(device('disk2', 'external', true)));
		expect(selectionStore.selected).toBeNull();
	});
});

describe('settingsStore', () => {
	it('読み込めなければ既定値、settings-changedで更新', async () => {
		api.getSettings.mockRejectedValue(new Error('no tauri'));
		const { settingsStore, DEFAULT_SETTINGS } = await load();
		await settingsStore.start();
		expect(settingsStore.settings).toEqual(DEFAULT_SETTINGS);
		emit('settings-changed', { ...DEFAULT_SETTINGS, language: 'en' });
		expect(settingsStore.settings.language).toBe('en');
	});

	it('updateは指定した項目だけを依頼し、結果を反映する', async () => {
		api.getSettings.mockResolvedValue({ language: 'system' });
		api.updateSettings.mockResolvedValue({ language: 'en' });
		const { settingsStore } = await load();
		await settingsStore.start();
		await settingsStore.update({ language: 'en' });
		expect(api.updateSettings).toHaveBeenCalledWith({ language: 'en' });
		expect(settingsStore.settings.language).toBe('en');
	});
});

describe('logStore', () => {
	it('読み込み・追記・消去。読み込み中に届いた記録は重複させない', async () => {
		api.getLog.mockResolvedValue([entry(1), entry(2)]);
		const { logStore } = await load();
		const starting = logStore.start();
		await Promise.resolve();
		emit('log-appended', entry(2)); // 取得結果に含まれる記録
		emit('log-appended', entry(3, 'osascript'));
		await starting;
		expect(logStore.entries.map((e) => e.at)).toEqual([1, 2, 3]);
		expect(logStore.last?.command).toBe('osascript');

		emit('log-appended', entry(4));
		expect(logStore.last?.at).toBe(4);
		emit('log-cleared');
		expect(logStore.entries).toEqual([]);
		expect(logStore.last).toBeNull();
	});

	it('上限を超えたら古いものから捨てる', async () => {
		api.getLog.mockResolvedValue([]);
		const { logStore, LOG_LIMIT } = await load();
		await logStore.start();
		for (let i = 0; i < LOG_LIMIT + 5; i++) emit('log-appended', entry(i));
		expect(logStore.entries).toHaveLength(LOG_LIMIT);
		expect(logStore.entries[0].at).toBe(5);
	});
});

describe('privilegedStore・toastStore', () => {
	it('補助プロセスの状態を受け取る', async () => {
		const { privilegedStore } = await load();
		await privilegedStore.start();
		expect(privilegedStore.status).toBe('none');
		emit('privileged-status', 'waiting_auth');
		expect(privilegedStore.status).toBe('waiting_auth');
	});

	it('通知を出して消す。時間で消す通知もある', async () => {
		vi.useFakeTimers();
		const { toastStore } = await load();
		const a = toastStore.push({ kind: 'ok', title: 'a' });
		toastStore.push({ kind: 'ok', title: 'b', timeoutMs: 1000 });
		expect(toastStore.items.map((t) => t.title)).toEqual(['a', 'b']);
		vi.advanceTimersByTime(1000);
		expect(toastStore.items.map((t) => t.title)).toEqual(['a']);
		toastStore.dismiss(a);
		expect(toastStore.items).toEqual([]);
		vi.useRealTimers();
	});
});
