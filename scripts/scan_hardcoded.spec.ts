import { describe, expect, it } from 'vitest';
import { scanCss, scanSvelte } from './scan_hardcoded';
import { latestVersion } from './check_copies';

describe('scanCss', () => {
	it('色とpxを見つけ、トークンの参照とコメントは無視する', () => {
		const css = ['/* #fff 1px */', '.a { color: #1f6fd1; margin: 4px; }', '.b { color: var(--cds-text-primary); background: rgb(0 0 0); }'].join('\n');
		expect(scanCss(css)).toEqual([
			{ line: 2, kind: '色', value: '#1f6fd1' },
			{ line: 2, kind: 'サイズ', value: '4px' },
			{ line: 3, kind: '色', value: 'rgb(' }
		]);
	});

	it('remや0、識別子の中の数字は対象外', () => {
		expect(scanCss('.a { padding: 0.5rem 0; grid-area: a1px; }')).toEqual([]);
	});
});

describe('scanSvelte', () => {
	it('<style>の中とstyle属性を調べる', () => {
		const source = ['<div style="width: 12px"></div>', '<style>', '  .a { color: #000; }', '</style>'].join('\n');
		expect(scanSvelte(source)).toEqual([
			{ line: 3, kind: '色', value: '#000' },
			{ line: 1, kind: 'サイズ', value: '12px' }
		]);
	});
});

describe('latestVersion', () => {
	it('版番号が最大のファイルを選ぶ', () => {
		const names = ['esp_latch_tokens_v2.css', 'esp_latch_tokens_v10.css', 'esp_latch_brand_v3.md'];
		expect(latestVersion(names, /^esp_latch_tokens_v(\d+)\.css$/)).toBe('esp_latch_tokens_v10.css');
	});
});
