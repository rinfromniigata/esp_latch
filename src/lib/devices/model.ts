// デバイス・パーティションの見方（UIの表示のための判定）。Rustのドメインモデルを読むだけで、変えない
import type { Device, DefaultFilter, DeviceSnapshot, Partition } from '#lib/api/types.js';

export function isEsp(partition: Partition): boolean {
	return partition.kind === 'esp';
}

export function espPartitions(device: Device): Partition[] {
	return device.partitions.filter(isEsp);
}

export function espCount(device: Device): number {
	return espPartitions(device).length;
}

export function mountedEspCount(device: Device): number {
	return espPartitions(device).filter((partition) => partition.mount_point !== null).length;
}

/** ESP以外のパーティション（その他のパーティションの表に出す） */
export function otherPartitions(device: Device): Partition[] {
	return device.partitions.filter((partition) => !isEsp(partition));
}

/** 起動に使ったESPか。OpenCoreの起動パスのUUIDと一致するESPだけ（Cloverや実機のMacでは印を出さない） */
export function isBootEsp(partition: Partition, snapshot: Pick<DeviceSnapshot, 'boot_part_uuid'>): boolean {
	return (
		isEsp(partition) &&
		partition.part_uuid !== null &&
		snapshot.boot_part_uuid !== null &&
		partition.part_uuid.toLowerCase() === snapshot.boot_part_uuid.toLowerCase()
	);
}

/** 絞り込み。all＝すべて、esp＝ESPのあるもの、physical＝ディスクイメージを除く */
export function filterDevices(devices: readonly Device[], filter: DefaultFilter): Device[] {
	switch (filter) {
		case 'esp':
			return devices.filter((device) => espCount(device) > 0);
		case 'physical':
			return devices.filter((device) => device.kind !== 'image');
		case 'all':
			return [...devices];
	}
}

/** サイドバーと詳細の見出しに出す名前。ラベルがなければベンダー名、それもなければ識別子 */
export function displayNames(device: Device): { main: string; vendor: string | null; more: number } {
	const main = device.labels[0] ?? device.vendor ?? device.id;
	// ラベルが先頭にあるとき、ベンダー名を小さく添える（ベンダー名が先頭になったら重ねない）
	const vendor = device.labels.length > 0 ? device.vendor : null;
	return { main, vendor, more: device.extra_label_count };
}

/** 帯グラフ・小さな帯の区画の色（var()の式）。マウント中のESPは状態の色 */
export function partitionColorVar(partition: Partition): string {
	if (isEsp(partition)) {
		return partition.mount_point !== null ? 'var(--esp-state-mounted)' : 'var(--esp-part-esp)';
	}
	return `var(--esp-part-${partition.kind})`;
}

/** 区画の幅の比率。小さな区画も見えるよう、下限を設ける（帯の最小幅） */
export function partitionWeight(partition: Partition, device: Pick<Device, 'size'>, floor: number): number {
	return Math.max(device.size > 0 ? partition.size / device.size : 0, floor);
}
