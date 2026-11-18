// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	interface ImportMetaEnv {
		/** 疑似ロケールで動かしているか（vite.config.tsのdefineで決まる。開発サーバーだけtrueになりうる） */
		readonly ESP_LATCH_PSEUDO_LOCALE: boolean;
	}
}

export {};
