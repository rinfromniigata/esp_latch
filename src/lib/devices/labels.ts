// デバイスまわりの表示名（辞書の文言）。辞書のキーは静的に書き、未使用キーの検査に見つかるようにする
import { m } from '#lib/i18n/index.js';
import type { DefaultFilter, DeviceKind, PartitionScheme } from '#lib/api/types.js';

export function kindLabel(kind: DeviceKind): string {
	switch (kind) {
		case 'internal':
			return m['device.kind.internal']();
		case 'external':
			return m['device.kind.external']();
		case 'image':
			return m['device.kind.image']();
	}
}

export function filterLabel(filter: DefaultFilter): string {
	switch (filter) {
		case 'all':
			return m['filter.all']();
		case 'esp':
			return m['filter.esp']();
		case 'physical':
			return m['filter.physical']();
	}
}

/** パーティション方式の表示。技術用語なのでロケールによらず同じ。なければnull */
export function schemeLabel(scheme: PartitionScheme): string | null {
	switch (scheme) {
		case 'gpt':
			return 'GPT';
		case 'mbr':
			return 'MBR';
		case 'apm':
			return 'APM';
		case 'none':
			return null;
	}
}

/** 値がないことを示す表示（形式・名前など。マウント先は辞書の`table.not_mounted`） */
export const EMPTY_VALUE = '—';
