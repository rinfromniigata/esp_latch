import { describe, expect, it } from 'vitest';
import { pseudoize, pseudoizeMessages } from './pseudo_locale';

describe('pseudoize', () => {
	it('［］で囲み、3〜4割伸ばす', () => {
		const result = pseudoize('ストレージ');
		expect(result.startsWith('［ストレージ')).toBe(true);
		expect(result.endsWith('］')).toBe(true);
		expect(result.length - 2).toBeGreaterThanOrEqual(Math.ceil(5 * 1.3));
		expect(result.length - 2).toBeLessThanOrEqual(Math.ceil(5 * 1.4));
	});

	it('プレースホルダは残し、伸ばす量の計算に含めない', () => {
		expect(pseudoize('{count}台')).toBe('［{count}台〜］');
	});
});

describe('pseudoizeMessages', () => {
	it('入れ子と複数形の各値を疑似化し、$schemaは残す', () => {
		const result = pseudoizeMessages({
			$schema: 'https://inlang.com/schema/inlang-message-format',
			toolbar: { refresh: '更新' },
			count: [{ selectors: ['countPlural'], match: { 'countPlural=other': '{count}個' } }]
		});
		expect(result).toEqual({
			$schema: 'https://inlang.com/schema/inlang-message-format',
			toolbar: { refresh: '［更新〜］' },
			count: [{ selectors: ['countPlural'], match: { 'countPlural=other': '［{count}個〜］' } }]
		});
	});
});
