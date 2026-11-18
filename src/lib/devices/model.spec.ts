import { describe, expect, it } from 'vitest';
import type { Device, Partition } from '#lib/api/types.js';
import { displayNames, espCount, filterDevices, isBootEsp, mountedEspCount, otherPartitions } from './model';

function partition(patch: Partial<Partition>): Partition {
	return {
		id: 'disk2s1',
		kind: 'esp',
		type_name: 'EFI',
		filesystem: 'FAT32',
		name: 'EFI',
		offset: null,
		size: 209_715_200,
		part_uuid: null,
		mount_point: null,
		apfs: null,
		...patch
	};
}

function device(patch: Partial<Device>): Device {
	return {
		id: 'disk2',
		kind: 'external',
		labels: [],
		extra_label_count: 0,
		vendor: null,
		image: null,
		connection: 'USB',
		scheme: 'gpt',
		size: 32_000_000_000,
		partitions: [],
		...patch
	};
}

const withEsp = device({
	id: 'disk2',
	partitions: [
		partition({ id: 'disk2s1', mount_point: '/Volumes/EFI' }),
		partition({ id: 'disk2s2', kind: 'hfs', name: 'Install' })
	]
});
const image = device({ id: 'disk6', kind: 'image', partitions: [partition({ id: 'disk6s1' })] });
const noEsp = device({ id: 'disk8', partitions: [partition({ id: 'disk8s1', kind: 'other' })] });

describe('ESPの数え方', () => {
	it('ESPとマウント中のESPを数え、ESP以外を別に返す', () => {
		expect(espCount(withEsp)).toBe(1);
		expect(mountedEspCount(withEsp)).toBe(1);
		expect(otherPartitions(withEsp).map((p) => p.id)).toEqual(['disk2s2']);
		expect(espCount(noEsp)).toBe(0);
	});
});

describe('起動に使用の印', () => {
	it('ESPで、OpenCoreの起動パスのUUIDと（大文字小文字を問わず）一致したときだけ', () => {
		const esp = partition({ part_uuid: 'AAAA-1111' });
		expect(isBootEsp(esp, { boot_part_uuid: 'aaaa-1111' })).toBe(true);
		expect(isBootEsp(esp, { boot_part_uuid: 'BBBB' })).toBe(false);
		expect(isBootEsp(esp, { boot_part_uuid: null })).toBe(false);
		expect(isBootEsp(partition({ part_uuid: null }), { boot_part_uuid: null })).toBe(false);
		expect(isBootEsp(partition({ kind: 'apfs', part_uuid: 'AAAA-1111' }), { boot_part_uuid: 'AAAA-1111' })).toBe(false);
	});
});

describe('絞り込み', () => {
	const all = [withEsp, image, noEsp];
	it('全て・ESPあり・物理', () => {
		expect(filterDevices(all, 'all').map((d) => d.id)).toEqual(['disk2', 'disk6', 'disk8']);
		expect(filterDevices(all, 'esp').map((d) => d.id)).toEqual(['disk2', 'disk6']);
		expect(filterDevices(all, 'physical').map((d) => d.id)).toEqual(['disk2', 'disk8']);
	});
});

describe('表示名', () => {
	it('ラベルを大きく、ベンダー名を小さく、残りのラベルの数を添える', () => {
		expect(displayNames(device({ labels: ['Macintosh HD'], extra_label_count: 2, vendor: 'Samsung SSD' }))).toEqual({
			main: 'Macintosh HD',
			vendor: 'Samsung SSD',
			more: 2
		});
	});
	it('ラベルがなければベンダー名を大きく出し、重ねて小さくは出さない', () => {
		expect(displayNames(device({ vendor: 'Kingston' }))).toEqual({ main: 'Kingston', vendor: null, more: 0 });
	});
	it('ラベルもベンダー名もなければ識別子', () => {
		expect(displayNames(device({}))).toEqual({ main: 'disk2', vendor: null, more: 0 });
	});
});
