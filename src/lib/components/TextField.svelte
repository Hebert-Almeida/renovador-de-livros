<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	let {
		label,
		value = $bindable(''),
		hint,
		error,
		id,
		type = 'text',
		...rest
	}: Omit<HTMLInputAttributes, 'value'> & { label: string; value?: string; hint?: string; error?: string } = $props();

	const uid = $props.id();
	const inputId = $derived(id ?? `tf-${uid}`);
	const describedBy = $derived([hint && `${inputId}-hint`, error && `${inputId}-err`].filter(Boolean).join(' ') || undefined);
</script>

<div class="field">
	<label class="field__label" for={inputId}>{label}</label>
	{#if hint}<p class="field__hint" id={`${inputId}-hint`}>{hint}</p>{/if}
	<input
		{...rest}
		{type}
		id={inputId}
		class="field__input"
		bind:value
		aria-invalid={error ? true : undefined}
		aria-describedby={describedBy}
	/>
	{#if error}<p class="field__error" id={`${inputId}-err`}>⚠ {error}</p>{/if}
</div>

<style>
	.field {
		display: grid;
		gap: var(--space-1);
	}
	.field__label {
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
	}
	.field__hint {
		margin: 0;
		color: var(--color-ink-muted);
		font-size: var(--text-sm);
	}
	.field__input {
		min-height: var(--target-min);
		padding: var(--space-2) var(--space-3);
		border: var(--border);
		border-radius: var(--radius);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		font: inherit;
		font-family: var(--font-mono);
		box-shadow: inset var(--shadow-offset) var(--shadow-offset) 0 var(--color-paper-sunken);
	}
	.field__input[aria-invalid='true'] {
		border-color: var(--color-danger);
	}
	.field__error {
		margin: 0;
		color: var(--color-danger);
		font-weight: 700;
		font-size: var(--text-sm);
	}
</style>
