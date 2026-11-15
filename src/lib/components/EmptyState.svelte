<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		description,
		headingLevel = 2,
		illustration,
		actions
	}: {
		title: string;
		description?: string;
		headingLevel?: 2 | 3 | 4;
		illustration?: Snippet;
		actions?: Snippet;
	} = $props();
</script>

<section class="empty">
	<div class="empty__art" aria-hidden="true">
		{#if illustration}
			{@render illustration()}
		{:else}
			<svg viewBox="0 0 96 64" width="96" height="64" fill="none" stroke="currentColor" stroke-width="3">
				<rect x="6" y="10" width="16" height="48" />
				<rect x="26" y="4" width="16" height="54" />
				<rect x="48" y="16" width="16" height="42" transform="rotate(-12 56 37)" />
				<line x1="0" y1="60" x2="96" y2="60" />
			</svg>
		{/if}
	</div>
	<svelte:element this={`h${headingLevel}`} class="empty__title">{title}</svelte:element>
	{#if description}<p class="empty__desc">{description}</p>{/if}
	{#if actions}<div class="empty__actions">{@render actions()}</div>{/if}
</section>

<style>
	.empty {
		display: grid;
		justify-items: center;
		gap: var(--space-2);
		padding: var(--space-5) var(--space-4);
		border: var(--border);
		border-style: dashed;
		text-align: center;
	}
	.empty__art {
		color: var(--color-ink);
	}
	.empty__title {
		margin: 0;
		font-size: var(--text-lg);
	}
	.empty__desc {
		margin: 0;
		max-width: 32rem;
		color: var(--color-ink-muted);
	}
	.empty__actions {
		display: flex;
		gap: var(--space-3);
		margin-top: var(--space-3);
	}
</style>
