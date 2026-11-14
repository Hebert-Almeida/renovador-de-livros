<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { press } from '#lib/motion/index.ts';

	let {
		label,
		children,
		type = 'button',
		onclick,
		class: className = '',
		...rest
	}: HTMLButtonAttributes & { label: string; children: Snippet } = $props();
</script>

<button
	{...rest}
	{type}
	class={['icon-btn', className]}
	aria-label={label}
	title={label}
	onclick={(e) => {
		press(e.currentTarget);
		onclick?.(e);
	}}
>
	<span aria-hidden="true">{@render children()}</span>
</button>

<style>
	.icon-btn {
		display: inline-grid;
		place-items: center;
		width: var(--target-min);
		height: var(--target-min);
		padding: 0;
		border: var(--border);
		border-radius: var(--radius);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		font-family: var(--font-display);
		font-size: var(--text-lg);
		font-weight: var(--weight-bold);
		box-shadow: var(--shadow-hard);
		cursor: pointer;
	}
	.icon-btn:active:not(:disabled) {
		transform: translate(var(--shadow-offset), var(--shadow-offset));
		box-shadow: none;
	}
	.icon-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
