<script lang="ts">
	type Tone = 1 | 2 | 3 | 4 | 5;
	let { title, author, due, href, tone }: { title: string; author?: string; due?: string; href?: string; tone?: Tone } =
		$props();

	// Cor derivada do título (capas geradas, sem baixar imagens).
	const t = $derived(tone ?? ((([...title].reduce((a, c) => a + c.charCodeAt(0), 0) % 5) + 1) as Tone));
</script>

{#if href}
	<a {href} class="spine" style:--spine-bg={`var(--spine-${t})`}>
		<span class="spine__title">{title}</span>
		{#if author}<span class="spine__author"><span class="visually-hidden">, </span>{author}</span>{/if}
		{#if due}<span class="spine__due"><span class="visually-hidden">, devolução </span>{due}</span>{/if}
	</a>
{:else}
	<div class="spine" style:--spine-bg={`var(--spine-${t})`}>
		<span class="spine__title">{title}</span>
		{#if author}<span class="spine__author">{author}</span>{/if}
		{#if due}<span class="spine__due">{due}</span>{/if}
	</div>
{/if}

<style>
	.spine {
		display: inline-flex;
		flex-direction: column;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-2);
		min-width: var(--target-min);
		width: 3.5rem;
		height: 15rem;
		padding: var(--space-3) var(--space-1);
		background: var(--spine-bg);
		color: var(--spine-ink);
		border: var(--border);
		box-shadow: var(--shadow-hard);
		text-decoration: none;
		transition: transform var(--duration-fast) var(--ease-out);
	}
	.spine__title,
	.spine__author {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-height: 9rem;
	}
	.spine__title {
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
	}
	.spine__author {
		font-size: var(--text-xs);
	}
	.spine__due {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		border-top: var(--border-width-thin) solid currentColor;
		padding-top: var(--space-1);
	}
	a.spine:hover {
		transform: translateY(-0.25rem);
	}
</style>
