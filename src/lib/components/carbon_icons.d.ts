// @carbon/icons は型定義を持たないため、アイコンの記述（svgの要素の木。components/icon.ts の IconDescriptor）の型を宣言する
declare module '@carbon/icons/es/*' {
	const icon: import('./icon').IconDescriptor;
	export default icon;
}
