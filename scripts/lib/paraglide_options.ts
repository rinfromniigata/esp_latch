// Paraglideのコンパイルの設定。vite.config.ts（開発サーバー・ビルド）とcheckコマンドで共有する
export const paraglideOptions = {
	project: './project.inlang',
	outdir: './src/lib/paraglide',
	// ロケールはアプリが決めて setLocale(…, { reload: false }) で渡す（src/lib/i18n/）
	strategy: ['globalVariable', 'baseLocale']
} as const satisfies { project: string; outdir: string; strategy: readonly string[] };
