<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		subtitle,
		headingLevel = 2,
		children,
		actions,
		stamps
	}: {
		title: string;
		subtitle?: string;
		headingLevel?: 2 | 3 | 4;
		children?: Snippet;
		actions?: Snippet;
		stamps?: Snippet;
	} = $props();
</script>

<article class="ficha">
	<header class="ficha__head">
		<svelte:element this={`h${headingLevel}`} class="ficha__title">{title}</svelte:element>
		{#if subtitle}<p class="ficha__subtitle">{subtitle}</p>{/if}
	</header>
	{#if children}<div class="ficha__body">{@render children()}</div>{/if}
	{#if stamps}<div class="ficha__stamps">{@render stamps()}</div>{/if}
	{#if actions}<footer class="ficha__actions">{@render actions()}</footer>{/if}
</article>

<style>
	.ficha {
		position: relative;
		background-color: var(--color-paper-raised);
		background-image: repeating-linear-gradient(
			transparent 0,
			transparent calc(var(--grid-size) - 1px),
			var(--grid-line) calc(var(--grid-size) - 1px),
			var(--grid-line) var(--grid-size)
		);
		border: var(--border);
		box-shadow: var(--shadow-hard-lg);
		padding: var(--space-4);
	}
	.ficha__head {
		border-bottom: var(--border);
		padding-bottom: var(--space-2);
		margin-bottom: var(--space-3);
	}
	.ficha__title {
		margin: 0;
		font-size: var(--text-lg);
	}
	.ficha__subtitle {
		margin: var(--space-1) 0 0;
		color: var(--color-ink-muted);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
	}
	.ficha__stamps {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
		margin-top: var(--space-3);
	}
	.ficha__actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin-top: var(--space-4);
	}
</style>
