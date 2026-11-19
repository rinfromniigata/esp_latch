import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { EspDetail, Partition } from '#lib/api/types.js';

// 本体（Tauri）の代わりに窓口を偽物にして、ESPのストアの動き（詳細の保持・通知・再試行）を確かめる
const api = vi.hoisted(() => ({
	inspectEsp: vi.fn(),
	mountEsp: vi.fn(),
	unmountEsp: vi.fn(),
	emptyEspTrash: vi.fn(),
	ejectDevice: vi.fn(),
	openEsp: vi.fn()
}));
const windows = vi.hoisted(() => ({ openAppWindow: vi.fn() }));

vi.mock('#lib/api/commands.js', () => api);
vi.mock('#lib/app/windows.js', () => windows);

function esp(id: string, mount_point: string | null): Partition {
	return {
		id,
		kind: 'esp',
		type_name: 'EFI',
		filesystem: 'FAT32',
		name: 'EFI',
		offset: null,
		size: 209_715_200,
		part_uuid: null,
		mount_point,
		apfs: null
	};
}

function detail(part: string, mount_point = '/Volumes/EFI'): EspDetail {
	return {
		part,
		mount_point,
		loader: { name: 'opencore', version: '1.0.8' },
		usage: { total: 209_715_200, free: 160_000_000, trash: 0 }
	};
}

async function load() {
	vi.resetModules();
	const [esp, toasts] = await Promise.all([import('./esp.svelte'), import('./toasts.svelte')]);
	return { espStore: esp.espStore, errorActions: esp.errorActions, toastStore: toasts.toastStore };
}

beforeEach(() => {
	for (const fn of [...Object.values(api), windows.openAppWindow]) fn.mockReset();
	vi.useRealTimers();
});

describe('espStore', () => {
	it('マウント中のESPだけ詳細を読み、取り外されたものは消す', async () => {
		const { espStore } = await load();
		api.inspectEsp.mockImplementation(async (part: string) => detail(part));
		await espStore.sync([esp('disk4s1', '/Volumes/EFI'), esp('disk6s1', null)]);
		expect(api.inspectEsp).toHaveBeenCalledTimes(1);
		expect(espStore.detail('disk4s1')?.loader?.version).toBe('1.0.8');
		expect(espStore.detail('disk6s1')).toBeUndefined();

		await espStore.sync([esp('disk4s1', null)]);
		expect(espStore.detail('disk4s1')).toBeUndefined();
	});

	it('読めなかった詳細は持たない', async () => {
		const { espStore } = await load();
		api.inspectEsp.mockRejectedValue({ kind: 'not_mounted' });
		await espStore.sync([esp('disk4s1', '/Volumes/EFI')]);
		expect(espStore.detail('disk4s1')).toBeUndefined();
	});

	it('マウントに成功したら詳細を持ち、数秒で消える成功の通知を出す', async () => {
		vi.useFakeTimers();
		const { espStore, toastStore } = await load();
		api.mountEsp.mockResolvedValue(detail('disk4s1'));
		await espStore.mount('disk4s1');
		expect(espStore.detail('disk4s1')?.mount_point).toBe('/Volumes/EFI');
		expect(toastStore.items).toHaveLength(1);
		expect(toastStore.items[0]?.kind).toBe('ok');
		expect(toastStore.items[0]?.body).toContain('/Volumes/EFI');
		vi.advanceTimersByTime(5000);
		expect(toastStore.items).toHaveLength(0);
	});

	it('認証のキャンセルは再試行のボタンつきで知らせ、押すと通知を閉じてやり直す', async () => {
		const { espStore, toastStore } = await load();
		api.mountEsp.mockRejectedValueOnce({ kind: 'auth_cancelled' });
		await espStore.mount('disk4s1');
		const toast = toastStore.items[0];
		expect(toast?.kind).toBe('error');
		expect(toast?.actions).toEqual(['retry']);

		api.mountEsp.mockResolvedValueOnce(detail('disk4s1'));
		toast?.onAction?.('retry');
		await vi.waitFor(() => expect(api.mountEsp).toHaveBeenCalledTimes(2));
		await vi.waitFor(() => expect(toastStore.items.map((t) => t.kind)).toEqual(['ok']));
	});

	it('マウントの失敗は、diskutilのエラーとコマンドを原文のまま出し、ログを開ける', async () => {
		const { espStore, toastStore } = await load();
		api.mountEsp.mockRejectedValue({
			kind: 'command_failed',
			command: 'diskutil mount disk4s1',
			stderr: 'Volume on disk4s1 failed to mount'
		});
		await espStore.mount('disk4s1');
		const toast = toastStore.items[0];
		expect(toast?.command).toBe('diskutil mount disk4s1');
		expect(toast?.stderr).toBe('Volume on disk4s1 failed to mount');
		expect(toast?.actions).toEqual(['retry', 'show_log']);
		toast?.onAction?.('show_log');
		expect(windows.openAppWindow).toHaveBeenCalledWith('log');
		expect(toastStore.items).toHaveLength(0);
	});

	it('取り外したら詳細を消す。失敗は通知する', async () => {
		const { espStore, toastStore } = await load();
		api.mountEsp.mockResolvedValue(detail('disk4s1'));
		await espStore.mount('disk4s1');
		toastStore.clear();
		api.unmountEsp.mockResolvedValueOnce(undefined);
		await espStore.unmount('disk4s1');
		expect(espStore.detail('disk4s1')).toBeUndefined();

		api.unmountEsp.mockRejectedValueOnce({ kind: 'busy' });
		await espStore.unmount('disk4s1');
		expect(toastStore.items[0]?.actions).toEqual([]);
	});

	it('「ゴミ箱を空にして取り外す」は、ゴミ箱を空にする指定つきで取り外す。失敗したら詳細を残して通知し、再試行できる', async () => {
		const { espStore, toastStore } = await load();
		api.mountEsp.mockResolvedValue(detail('disk4s1'));
		await espStore.mount('disk4s1');
		toastStore.clear();

		api.unmountEsp.mockRejectedValueOnce({ kind: 'io', message: 'Read-only file system' });
		await espStore.unmount('disk4s1', true);
		expect(api.unmountEsp).toHaveBeenLastCalledWith('disk4s1', true);
		expect(espStore.detail('disk4s1')).toBeDefined();
		expect(toastStore.items[0]?.kind).toBe('error');

		api.unmountEsp.mockResolvedValueOnce(undefined);
		toastStore.items[0]?.onAction?.('retry');
		await vi.waitFor(() => expect(api.unmountEsp).toHaveBeenCalledTimes(2));
		expect(api.unmountEsp).toHaveBeenLastCalledWith('disk4s1', true);
		await vi.waitFor(() => expect(espStore.detail('disk4s1')).toBeUndefined());
	});

	it('ふつうの取り外しは、ゴミ箱を空にする指定をしない（設定の自動削除は本体が見る）', async () => {
		const { espStore } = await load();
		api.unmountEsp.mockResolvedValue(undefined);
		await espStore.unmount('disk4s1');
		expect(api.unmountEsp).toHaveBeenCalledWith('disk4s1', false);
	});

	it('ゴミ箱を空にしたら、解放した容量を数秒で消える成功の通知で知らせる', async () => {
		vi.useFakeTimers();
		const { espStore, toastStore } = await load();
		api.emptyEspTrash.mockResolvedValue(158_564_352);
		await espStore.emptyTrash('disk4s1');
		const toast = toastStore.items[0];
		expect(toast?.kind).toBe('ok');
		expect(toast?.body).toContain('disk4s1');
		expect(toast?.body).toContain('159');
		vi.advanceTimersByTime(5000);
		expect(toastStore.items).toHaveLength(0);
	});

	it('ゴミ箱を空にできなければ通知し、コマンドの失敗はログを開ける', async () => {
		const { espStore, toastStore } = await load();
		api.emptyEspTrash.mockRejectedValue({
			kind: 'command_failed',
			command: 'rm -rf /Volumes/EFI/.Trashes',
			stderr: 'Operation not permitted'
		});
		await espStore.emptyTrash('disk4s1');
		expect(toastStore.items[0]?.kind).toBe('error');
		expect(toastStore.items[0]?.stderr).toBe('Operation not permitted');
		expect(toastStore.items[0]?.actions).toEqual(['retry', 'show_log']);
	});

	it('取り出したら、そのデバイスのESPの詳細を消す（ほかのデバイスのものは残す）', async () => {
		const { espStore, toastStore } = await load();
		api.inspectEsp.mockImplementation(async (part: string) => detail(part));
		await espStore.sync([esp('disk4s1', '/Volumes/EFI'), esp('disk40s1', '/Volumes/EFI 1')]);
		api.ejectDevice.mockResolvedValue(undefined);
		await espStore.eject('disk4');
		expect(api.ejectDevice).toHaveBeenCalledWith('disk4');
		expect(espStore.detail('disk4s1')).toBeUndefined();
		expect(espStore.detail('disk40s1')).toBeDefined();
		expect(toastStore.items).toHaveLength(0);
	});

	it('取り出せなければ、diskutilのエラーを通知して再試行できる', async () => {
		const { espStore, toastStore } = await load();
		api.ejectDevice.mockRejectedValueOnce({
			kind: 'command_failed',
			command: 'diskutil eject disk4',
			stderr: 'Volume disk4s2 failed to unmount'
		});
		await espStore.eject('disk4');
		const toast = toastStore.items[0];
		expect(toast?.command).toBe('diskutil eject disk4');
		api.ejectDevice.mockResolvedValueOnce(undefined);
		toast?.onAction?.('retry');
		await vi.waitFor(() => expect(api.ejectDevice).toHaveBeenCalledTimes(2));
		expect(toastStore.items).toHaveLength(0);
	});

	it('Finderで開けなければ通知する', async () => {
		const { espStore, toastStore } = await load();
		api.openEsp.mockRejectedValue({ kind: 'not_mounted' });
		await espStore.open('disk4s1');
		expect(toastStore.items[0]?.kind).toBe('error');
	});

	it('エラーの種別ごとのボタン', async () => {
		const { errorActions } = await load();
		expect(errorActions({ kind: 'auth_cancelled' })).toEqual(['retry']);
		expect(errorActions({ kind: 'command_failed', command: 'x', stderr: 'y' })).toEqual(['retry', 'show_log']);
		expect(errorActions({ kind: 'helper_unavailable' })).toEqual([]);
	});
});
