import { describe, expect, it } from 'vitest';
import { setLocale } from '#lib/paraglide/runtime.js';
import { describeError, toAppError } from './errors';
import type { AppError } from './types';

setLocale('ja', { reload: false });

describe('toAppError', () => {
	it('Rustから届くエラーの種別をそのまま受け取る', () => {
		expect(toAppError({ kind: 'busy' })).toEqual({ kind: 'busy' });
		expect(toAppError({ kind: 'command_failed', command: 'diskutil mount disk4s1', stderr: 'x' })).toEqual({
			kind: 'command_failed',
			command: 'diskutil mount disk4s1',
			stderr: 'x'
		});
	});
	it('知らない形は入出力のエラーにする', () => {
		expect(toAppError('boom')).toEqual({ kind: 'io', message: 'boom' });
		expect(toAppError({ kind: 'unknown' })).toMatchObject({ kind: 'io' });
		expect(toAppError(new Error('x'))).toEqual({ kind: 'io', message: 'x' });
	});
});

describe('describeError', () => {
	const kinds: AppError[] = [
		{ kind: 'auth_cancelled' },
		{ kind: 'command_failed', command: 'diskutil mount disk4s1', stderr: 'failed' },
		{ kind: 'not_esp' },
		{ kind: 'invalid_target' },
		{ kind: 'busy' },
		{ kind: 'helper_unavailable' },
		{ kind: 'not_mounted' },
		{ kind: 'io', message: 'secret detail' }
	];

	it('すべての種別に、見出しの文言がある', () => {
		for (const error of kinds) {
			const { title } = describeError(error);
			expect(title, error.kind).not.toBe('');
			expect(title, error.kind).not.toContain('⟦');
		}
	});

	it('マウントの失敗は専用の見出しにし、コマンドと標準エラー出力は原文のまま返す', () => {
		const message = describeError(kinds[1], 'mount_esp');
		expect(message.title).toBe('マウントできませんでした');
		expect(message.command).toBe('diskutil mount disk4s1');
		expect(message.stderr).toBe('failed');
		expect(describeError(kinds[1], 'unmount_esp').title).not.toBe(message.title);
	});

	it('入出力のエラーの調査用のメッセージは、利用者に見せない', () => {
		const message = describeError(kinds[7]);
		expect(JSON.stringify(message)).not.toContain('secret detail');
	});
});
