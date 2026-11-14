<script lang="ts" module>
	export interface SelectOption {
		value: string;
		label: string;
		disabled?: boolean;
	}
</script>

<script lang="ts">
	import type { HTMLSelectAttributes } from 'svelte/elements';

	let {
		label,
		options,
		value = $bindable(''),
		hint,
		id,
		...rest
	}: Omit<HTMLSelectAttributes, 'value'> & { label: string; options: SelectOption[]; value?: string; hint?: string } =
		$props();
	const uid = $props.id();
	const selectId = $derived(id ?? `sel-${uid}`);
</script>

<div class="field">
	<label class="field__label" for={selectId}>{label}</label>
	{#if hint}<p class="field__hint" id={`${selectId}-hint`}>{hint}</p>{/if}
	<div class="field__wrap">
		<select {...rest} id={selectId} class="field__select" bind:value aria-describedby={hint ? `${selectId}-hint` : undefined}>
			{#each options as o (o.value)}
				<option value={o.value} disabled={o.disabled}>{o.label}</option>
			{/each}
		</select>
		<span class="field__chev" aria-hidden="true">▼</span>
	</div>
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
	.field__wrap {
		position: relative;
	}
	.field__select {
		appearance: none;
		width: 100%;
		min-height: var(--target-min);
		padding: var(--space-2) calc(var(--space-5) + var(--space-2)) var(--space-2) var(--space-3);
		border: var(--border);
		border-radius: var(--radius);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		font: inherit;
		box-shadow: var(--shadow-hard);
		cursor: pointer;
	}
	.field__chev {
		position: absolute;
		right: var(--space-3);
		top: 50%;
		transform: translateY(-50%);
		pointer-events: none;
		font-size: var(--text-xs);
	}
</style>
