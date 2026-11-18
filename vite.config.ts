import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { buildPseudoProject } from './scripts/pseudo_locale.ts';
import { paraglideOptions } from './scripts/lib/paraglide_options.ts';

// Tauriの開発サーバーの設定（https://v2.tauri.app/start/frontend/sveltekit/）
const host = process.env.TAURI_DEV_HOST;

const CARBON_INTERACTIVE_WARNINGS = new Set([
	'a11y_click_events_have_key_events',
	'a11y_no_static_element_interactions'
]);

export default defineConfig(({ command }) => {
	// 疑似ロケールは開発サーバーだけで有効にする（公開版のビルドには含めない）
	const pseudo = command === 'serve' && process.env.ESP_LATCH_PSEUDO_LOCALE === '1';

	return {
		plugins: [
			paraglideVitePlugin({
				...paraglideOptions,
				project: pseudo ? buildPseudoProject(import.meta.dirname) : paraglideOptions.project,
				strategy: [...paraglideOptions.strategy]
			}),
			sveltekit({
				compilerOptions: {
					// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
					runes: ({ filename }) =>
						filename.split(/[/\\]/).includes('node_modules') ? undefined : true,
					// Carbonの部品（cds-*）は、Shadow DOMの中の<button>などがキーボード操作とロールを受け持つため、
					// 「クリックにキー操作が要る」「ロールが要る」の警告は誤検知になる。cds-* に限って外す
					warningFilter: (warning) =>
						!(
							CARBON_INTERACTIVE_WARNINGS.has(warning.code) && /`<cds-[\w-]+>`/.test(warning.message)
						)
				},
				// SPA：すべてのルートをindex.htmlで受ける（+layout.tsでssr=false）
				adapter: adapter({ fallback: 'index.html' })
			})
		],
		// 疑似ロケールのときは、OSや設定の言語にかかわらずja（疑似化した文言）で表示する
		define: { 'import.meta.env.ESP_LATCH_PSEUDO_LOCALE': JSON.stringify(pseudo) },
		clearScreen: false,
		server: {
			port: 1420,
			strictPort: true,
			host: host || false,
			hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
			watch: { ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**'] }
		},
		test: {
			expect: { requireAssertions: true },
			projects: [
				{
					extends: './vite.config.ts',
					test: {
						name: 'unit',
						environment: 'node',
						include: ['src/**/*.{test,spec}.{js,ts}', 'scripts/**/*.{test,spec}.ts'],
						exclude: ['src/**/*.svelte.{test,spec}.{js,ts}']
					}
				}
			]
		}
	};
});
