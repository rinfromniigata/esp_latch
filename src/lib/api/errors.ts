// AppError（Rustのエラーの種別）から、通知に出す文言への対応。文言は辞書から引き、種別ごとに辞書のキーを決める。
// command_failedの標準エラー出力とコマンドは、原文のまま見せる（imple 1章）
import { m } from '#lib/i18n/index.js';
import type { AppError, Operation } from './types';

const KINDS = [
	'auth_cancelled',
	'command_failed',
	'not_esp',
	'invalid_target',
	'busy',
	'helper_unavailable',
	'not_mounted',
	'io'
] as const;

/** invokeが拒否した値（Rustから届くAppError）をAppErrorに直す。形が違うものは、種別を判別できない入出力のエラーにする */
export function toAppError(value: unknown): AppError {
	if (typeof value === 'object' && value !== null && 'kind' in value) {
		const kind = (value as { kind: unknown }).kind;
		if (typeof kind === 'string' && (KINDS as readonly string[]).includes(kind)) {
			return value as AppError;
		}
	}
	return { kind: 'io', message: value instanceof Error ? value.message : String(value) };
}

export interface ErrorMessage {
	title: string;
	/** 辞書の文言の本文。ない種別もある */
	body?: string;
	/** 実行したコマンドの原文 */
	command?: string;
	/** 標準エラー出力の原文 */
	stderr?: string;
}

/** AppErrorを通知の文言にする。opは、command_failedの見出しを操作に合わせるために使う（マウントだけ専用の見出し） */
export function describeError(error: AppError, op?: Operation): ErrorMessage {
	switch (error.kind) {
		case 'auth_cancelled':
			return { title: m['toast.auth_cancelled.title'](), body: m['toast.auth_cancelled.body']() };
		case 'command_failed':
			return {
				title: op === 'mount_esp' ? m['toast.mount_failed.title']() : m['error.command_failed.title'](),
				command: error.command,
				stderr: error.stderr
			};
		case 'not_esp':
			return { title: m['error.not_esp.title'](), body: m['error.not_esp.body']() };
		case 'invalid_target':
			return { title: m['error.invalid_target.title'](), body: m['error.invalid_target.body']() };
		case 'busy':
			return { title: m['error.busy.title'](), body: m['error.busy.body']() };
		case 'helper_unavailable':
			return { title: m['error.helper_unavailable.title'](), body: m['error.helper_unavailable.body']() };
		case 'not_mounted':
			return { title: m['error.not_mounted.title'](), body: m['error.not_mounted.body']() };
		case 'io':
			// messageは調査用で、利用者には見せない
			return { title: m['error.io.title'](), body: m['error.io.body']() };
	}
}
