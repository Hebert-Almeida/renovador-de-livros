<script lang="ts">
	import { onMount } from 'svelte';
	import { stamp } from '#lib/motion/index.ts';

	/** `date` no formato ISO `AAAA-MM-DD`. */
	let { date, label = 'Renovado', animate = false }: { date: string; label?: string; animate?: boolean } = $props();
	let el: HTMLElement | undefined = $state();
	const shown = $derived.by(() => {
		const [y, m, d] = date.split('-');
		return d && m && y ? `${d}/${m}/${y}` : date;
	});

	onMount(() => {
		if (animate && el) void stamp(el);
	});
</script>

<span class="stamp" bind:this={el}>
	<span class="stamp__label">{label}</span>
	<time class="stamp__date" datetime={date}>{shown}</time>
</span>

<style>
	.stamp {
		display: inline-flex;
		flex-direction: column;
		align-items: center;
		padding: var(--space-1) var(--space-3);
		border: var(--border-width) double var(--color-stamp);
		outline: var(--border-width-thin) solid var(--color-stamp);
		outline-offset: 0.125rem;
		color: var(--color-stamp);
		transform: rotate(-6deg);
		text-transform: uppercase;
		line-height: 1.2;
	}
	.stamp__label {
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		font-size: var(--text-xs);
		letter-spacing: var(--tracking-caps);
	}
	.stamp__date {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 700;
	}
</style>
