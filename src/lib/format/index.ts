// 日付・時刻・数値・容量の整形。必ずロケールを渡してIntlで整形し、文字列の連結で組み立てない
import { getLocale, type Locale } from '#lib/paraglide/runtime.js';

const BYTE_UNITS = [
	['terabyte', 1e12],
	['gigabyte', 1e9],
	['megabyte', 1e6],
	['kilobyte', 1e3]
] as const;

/**
 * 容量を10進の単位（KB・MB・GB・TB。diskutil・Finderと同じ）で整形する。
 * 100以上は整数、それ未満は小数1桁まで。
 */
export function formatBytes(bytes: number, locale: Locale = getLocale()): string {
	for (const [unit, size] of BYTE_UNITS) {
		if (bytes >= size) {
			const value = bytes / size;
			return new Intl.NumberFormat(locale, {
				style: 'unit',
				unit,
				unitDisplay: 'short',
				maximumFractionDigits: value >= 100 ? 0 : 1
			}).format(value);
		}
	}
	return new Intl.NumberFormat(locale, { style: 'unit', unit: 'byte', unitDisplay: 'short' }).format(
		bytes
	);
}

/** 時刻（24時間制、時・分・秒）。操作ログの時刻に使う */
export function formatTime(date: Date, locale: Locale = getLocale()): string {
	return new Intl.DateTimeFormat(locale, {
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
		hourCycle: 'h23'
	}).format(date);
}

/** 割合（0〜1）を百分率で整形する */
export function formatPercent(ratio: number, locale: Locale = getLocale()): string {
	return new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 0 }).format(ratio);
}

/** 個数などの整数 */
export function formatInteger(value: number, locale: Locale = getLocale()): string {
	return new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(value);
}
