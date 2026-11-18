// Carbonの部品は、Shadow DOMの中のCSSに書体名`IBM Plex Sans`を直書きしており（外から差し替えられない）、
// 和文は、書体の候補の残り（system-ui）で出てしまう。日本語のときだけ、`IBM Plex Sans`という名前で
// 和文の範囲に`IBM Plex Sans JP`を割り当て（FontFace）、Carbonの部品の和文も同じ書体にする。
// 中国語（簡体）は、tokens.cssの方針どおりOSの書体（PingFang）に任せるため、割り当てない。
import type { Locale } from '#lib/paraglide/runtime.js';
import regular from '@ibm/plex-sans-jp/fonts/complete/woff2/hinted/IBMPlexSansJP-Regular.woff2?url';
import medium from '@ibm/plex-sans-jp/fonts/complete/woff2/hinted/IBMPlexSansJP-Medium.woff2?url';
import semibold from '@ibm/plex-sans-jp/fonts/complete/woff2/hinted/IBMPlexSansJP-SemiBold.woff2?url';

/** 和文の範囲：全角の記号・かな・漢字・全角形 */
const JAPANESE_RANGE = 'U+3000-30FF, U+3400-4DBF, U+4E00-9FFF, U+F900-FAFF, U+FF00-FFEF';

const SOURCES: [url: string, weight: string][] = [
	[regular, '400'],
	[medium, '500'],
	[semibold, '600']
];

let faces: FontFace[] = [];

export function applyCarbonFonts(locale: Locale): void {
	if (typeof FontFace === 'undefined' || typeof document === 'undefined') return;
	const wanted = locale === 'ja';
	if (wanted === faces.length > 0) return;

	for (const face of faces) document.fonts.delete(face);
	faces = [];
	if (!wanted) return;

	faces = SOURCES.map(([url, weight]) => {
		const face = new FontFace('IBM Plex Sans', `url(${url}) format('woff2')`, {
			weight,
			style: 'normal',
			unicodeRange: JAPANESE_RANGE,
			display: 'block'
		});
		document.fonts.add(face);
		void face.load().catch(() => undefined);
		return face;
	});
}
