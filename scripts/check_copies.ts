// コピーの一致の検査（imple 9章の6）：src/lib/theme/tokens.css と docs/brand/ のファイルが、
// roadmap/specs/ の最新版（版番号が最大のもの）と同じ内容であることを確かめる
import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

/** 正本（specs/の名前の型）と、そのコピーの置き場所 */
const COPIES = [
	{ pattern: /^esp_latch_tokens_v(\d+)\.css$/, copy: 'src/lib/theme/tokens.css' },
	{ pattern: /^esp_latch_brand_v(\d+)\.md$/, copy: 'docs/brand/brand.md' },
	{ pattern: /^esp_latch_icon_master_v(\d+)\.svg$/, copy: 'docs/brand/icon_master.svg' }
] as const;

/** ファイル名の一覧から、版番号が最大のものを選ぶ */
export function latestVersion(names: readonly string[], pattern: RegExp): string | undefined {
	let best: { name: string; version: number } | undefined;
	for (const name of names) {
		const match = pattern.exec(name);
		if (!match) continue;
		const version = Number(match[1]);
		if (!best || version > best.version) best = { name, version };
	}
	return best?.name;
}

function main(): number {
	const root = resolve(import.meta.dirname, '..');
	const specsDir = resolve(root, 'roadmap/specs');
	const names = readdirSync(specsDir);
	const errors: string[] = [];

	for (const { pattern, copy } of COPIES) {
		const latest = latestVersion(names, pattern);
		if (!latest) {
			errors.push(`roadmap/specs/ に正本がありません（${pattern.source}）`);
			continue;
		}
		const copyPath = resolve(root, copy);
		if (!existsSync(copyPath)) {
			errors.push(`${copy} がありません（roadmap/specs/${latest} をコピーしてください）`);
			continue;
		}
		if (!readFileSync(copyPath).equals(readFileSync(resolve(specsDir, latest)))) {
			errors.push(`${copy} が roadmap/specs/${latest} と一致しません（コピーし直してください。直接編集しません）`);
		}
	}

	if (errors.length > 0) {
		console.error(errors.join('\n'));
		return 1;
	}
	return 0;
}

if (import.meta.main) process.exit(main());
