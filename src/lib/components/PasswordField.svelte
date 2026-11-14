<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	let {
		label,
		value = $bindable(''),
		hint,
		error,
		id,
		autocomplete = 'current-password',
		...rest
	}: Omit<HTMLInputAttributes, 'value' | 'type'> & {
		label: string;
		value?: string;
		hint?: string;
		error?: string;
	} = $props();

	const uid = $props.id();
	const inputId = $derived(id ?? `pw-${uid}`);
	let visible = $state(false);
	const describedBy = $derived([hint && `${inputId}-hint`, error && `${inputId}-err`].filter(Boolean).join(' ') || undefined);
</script>

<div class="field">
	<label class="field__label" for={inputId}>{label}</label>
	{#if hint}<p class="field__hint" id={`${inputId}-hint`}>{hint}</p>{/if}
	<div class="field__row">
		<input
			{...rest}
			id={inputId}
			class="field__input"
			type={visible ? 'text' : 'password'}
			{autocomplete}
			spellcheck="false"
			bind:value
			aria-invalid={error ? true : undefined}
			aria-describedby={describedBy}
		/>
		<button
			type="button"
			class="field__reveal"
			aria-pressed={visible}
			aria-controls={inputId}
			aria-label={visible ? 'Ocultar senha' : 'Mostrar senha'}
			onclick={() => (visible = !visible)}
		>
			{visible ? 'Ocultar' : 'Mostrar'}
		</button>
	</div>
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
	.field__row {
		display: flex;
	}
	.field__input {
		flex: 1;
		min-width: 0;
		min-height: var(--target-min);
		padding: var(--space-2) var(--space-3);
		border: var(--border);
		border-right: 0;
		border-radius: var(--radius);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		font: inherit;
		font-family: var(--font-mono);
	}
	.field__input[aria-invalid='true'] {
		border-color: var(--color-danger);
	}
	.field__reveal {
		min-height: var(--target-min);
		min-width: var(--target-min);
		padding: 0 var(--space-3);
		border: var(--border);
		background: var(--color-paper-sunken);
		color: var(--color-ink);
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		cursor: pointer;
	}
	.field__error {
		margin: 0;
		color: var(--color-danger);
		font-weight: 700;
		font-size: var(--text-sm);
	}
</style>
