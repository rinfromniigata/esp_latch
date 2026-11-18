import { describe, expect, it } from 'vitest';
import { formatBytes, formatPercent, formatTime } from './index';

describe('formatBytes', () => {
	it('10進の単位を選び、100未満は小数1桁まで', () => {
		expect(formatBytes(15_500_000, 'en')).toBe('15.5 MB');
		expect(formatBytes(209_715_200, 'en')).toBe('210 MB');
		expect(formatBytes(1_500_000_000, 'en')).toBe('1.5 GB');
		expect(formatBytes(500_277_790_720, 'en')).toBe('500 GB');
	});

	it('1KB未満はバイトで出す', () => {
		expect(formatBytes(512, 'en')).toBe('512 byte');
	});

	it('ロケールの数値の書式に従う', () => {
		expect(formatBytes(1_500_000_000, 'ja')).toMatch(/^1\.5\s?GB$/);
	});
});

describe('formatTime', () => {
	it('24時間制で時・分・秒を出す', () => {
		expect(formatTime(new Date(2026, 10, 18, 9, 5, 7), 'en')).toBe('09:05:07');
		expect(formatTime(new Date(2026, 10, 18, 21, 5, 7), 'ja')).toBe('21:05:07');
	});
});

describe('formatPercent', () => {
	it('百分率で整数に丸める', () => {
		expect(formatPercent(0.876, 'en')).toBe('88%');
	});
});
