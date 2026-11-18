<script lang="ts">
	// Carbonのアイコンを表示する。意味を持つアイコンは、周りの文字やtitleで補う（アイコンだけで伝えない）
	import type { IconDescriptor, IconNode } from './icon';

	interface Props {
		icon: IconDescriptor;
		/** 表示の大きさ。既定は1rem（16相当） */
		size?: string;
		class?: string;
	}

	let { icon, size = '1rem', class: className = '' }: Props = $props();
</script>

{#snippet node(item: IconNode)}
	<!-- snippetの中ではSVGの名前空間が引き継がれないため、xmlnsで明示する（しないとHTMLの要素になり、何も描かれない） -->
	<svelte:element this={item.elem} xmlns="http://www.w3.org/2000/svg" {...item.attrs}>
		{#each item.content ?? [] as child, index (index)}
			{@render node(child)}
		{/each}
	</svelte:element>
{/snippet}

<svg
	class={['icon', className]}
	viewBox={icon.attrs.viewBox}
	width={size}
	height={size}
	fill="currentColor"
	aria-hidden="true"
	focusable="false"
>
	{#each icon.content ?? [] as child, index (index)}
		{@render node(child)}
	{/each}
</svg>

<style>
	.icon {
		flex: none;
		display: block;
	}
</style>
