<script lang="ts">
	// 空の状態：アイコン・見出し・説明、必要ならボタン（children）
	import type { Snippet } from 'svelte';
	import type { IconDescriptor } from './icon';
	import Icon from './Icon.svelte';

	interface Props {
		icon: IconDescriptor;
		title: string;
		body: string;
		children?: Snippet;
	}

	let { icon, title, body, children }: Props = $props();
</script>

<div class="empty">
	<Icon {icon} size="3rem" class="art" />
	<h2>{title}</h2>
	<p>{body}</p>
	{#if children}<div class="action">{@render children()}</div>{/if}
</div>

<style>
	.empty {
		margin: auto;
		text-align: center;
		max-width: 22.5rem;
		padding: var(--cds-spacing-07);
	}

	.empty :global(.art) {
		margin: 0 auto;
		color: var(--cds-icon-secondary);
	}

	h2 {
		margin: var(--cds-spacing-04) 0 var(--cds-spacing-02);
		font-size: var(--cds-heading-compact-02-font-size);
		font-weight: 500;
	}

	p {
		margin: 0;
		font-size: var(--cds-body-compact-01-font-size);
		color: var(--cds-text-secondary);
	}

	.action {
		margin-top: var(--cds-spacing-05);
	}
</style>
