<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { press } from '#lib/motion/index.ts';

	type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
	let {
		variant = 'secondary',
		type = 'button',
		loading = false,
		disabled = false,
		children,
		onclick,
		class: className = '',
		...rest
	}: HTMLButtonAttributes & { variant?: Variant; loading?: boolean; children: Snippet } = $props();
</script>

<button
	{...rest}
	{type}
	class={['btn', `btn--${variant}`, className]}
	disabled={disabled || loading}
	aria-busy={loading || undefined}
	onclick={(e) => {
		press(e.currentTarget);
		onclick?.(e);
	}}
>
	{@render children()}
</button>

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		min-height: var(--target-min);
		min-width: var(--target-min);
		padding: var(--space-2) var(--space-4);
		border: var(--border);
		border-radius: var(--radius);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		font-family: var(--font-display);
		font-size: var(--text-md);
		font-weight: var(--weight-bold);
		letter-spacing: 0.02em;
		box-shadow: var(--shadow-hard);
		cursor: pointer;
		transition:
			transform var(--duration-fast) var(--ease-out),
			box-shadow var(--duration-fast) var(--ease-out);
	}
	.btn:hover:not(:disabled) {
		transform: translate(-0.0625rem, -0.0625rem);
		box-shadow: var(--shadow-hard-lg);
	}
	.btn:active:not(:disabled) {
		transform: translate(var(--shadow-offset), var(--shadow-offset));
		box-shadow: none;
	}
	.btn--primary {
		background: var(--color-accent);
		color: var(--color-accent-ink);
	}
	.btn--danger {
		background: var(--color-danger-bg);
		color: var(--color-danger);
	}
	.btn--ghost {
		background: transparent;
		box-shadow: none;
		border-color: transparent;
		text-decoration: underline;
		text-underline-offset: 0.2em;
	}
	.btn:disabled {
		cursor: not-allowed;
		opacity: 0.6;
		box-shadow: none;
	}
</style>
