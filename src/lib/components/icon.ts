// Carbonのアイコン（@carbon/icons）の記述（svgの要素の木）
export interface IconNode {
	elem: string;
	attrs?: Record<string, string | number>;
	content?: IconNode[];
}

export interface IconDescriptor extends IconNode {
	attrs: Record<string, string | number> & { viewBox: string };
}
