import { readdirSync } from 'node:fs';
import { join } from 'node:path';

/** ディレクトリの下にある、指定した拡張子のファイルを再帰的に集める */
export function listFiles(dir: string, extensions: readonly string[]): string[] {
	const out: string[] = [];
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) out.push(...listFiles(path, extensions));
		else if (extensions.some((extension) => entry.name.endsWith(extension))) out.push(path);
	}
	return out.sort();
}
