// Rustのドメインモデル・イベント・エラー・設定の型（src-tauri/src/domain・events・oplog・app/error・settings と一致させる）
import type { LanguagePreference } from '#lib/i18n/resolve.js';
import type { ThemePreference } from '#lib/theme/theme.js';

export type DeviceKind = 'internal' | 'external' | 'image';
export type PartitionScheme = 'gpt' | 'mbr' | 'apm' | 'none';
export type PartitionKind = 'esp' | 'apfs' | 'hfs' | 'other';

export interface DiskImage {
	path: string;
	/** イメージファイルを保存しているボリュームのBSD名 */
	stored_on: string | null;
}

export interface ApfsVolume {
	id: string;
	name: string;
	mount_point: string | null;
	used: number | null;
}

export interface ApfsContainer {
	id: string;
	volumes: ApfsVolume[];
}

export interface Partition {
	id: string;
	kind: PartitionKind;
	/** diskutilのパーティションの種別名（EFI・Apple_APFSなど） */
	type_name: string;
	/** ファイルシステムの短い表示名（FAT32・APFSなど） */
	filesystem: string | null;
	name: string | null;
	offset: number | null;
	size: number;
	part_uuid: string | null;
	mount_point: string | null;
	apfs: ApfsContainer | null;
}

export interface Device {
	id: string;
	kind: DeviceKind;
	/** 表示名の候補。先頭が大きく出る名前。空ならvendorを大きく出す */
	labels: string[];
	extra_label_count: number;
	vendor: string | null;
	image: DiskImage | null;
	/** 接続の種別（diskutilの原文） */
	connection: string | null;
	scheme: PartitionScheme;
	size: number;
	partitions: Partition[];
}

export interface DeviceSnapshot {
	devices: Device[];
	boot_part_uuid: string | null;
	/** UNIX時刻のミリ秒 */
	taken_at: number;
}

export type LogResult = 'ok' | 'fail' | 'cancelled';

export interface LogEntry {
	/** UNIX時刻のミリ秒 */
	at: number;
	command: string;
	result: LogResult;
}

export type Operation = 'mount_esp' | 'unmount_esp' | 'open_esp' | 'eject_device' | 'empty_esp_trash';

export interface BusyState {
	target: string;
	op: Operation;
	active: boolean;
}

/** コマンドのエラー。kindを辞書のキーに対応づける */
export type AppError =
	| { kind: 'auth_cancelled' }
	| { kind: 'command_failed'; command: string; stderr: string }
	| { kind: 'not_esp' }
	| { kind: 'invalid_target' }
	| { kind: 'busy' }
	| { kind: 'helper_unavailable' }
	| { kind: 'not_mounted' }
	| { kind: 'io'; message: string };

export type DefaultFilter = 'all' | 'esp' | 'physical';

export interface Settings {
	language: LanguagePreference;
	theme: ThemePreference;
	default_filter: DefaultFilter;
	open_finder_after_mount: boolean;
	auto_refresh: boolean;
	confirm_eject: boolean;
	auto_empty_trash_on_unmount: boolean;
	show_command: boolean;
}

/** 設定の変更。指定した項目だけを変える */
export type SettingsPatch = Partial<Settings>;
