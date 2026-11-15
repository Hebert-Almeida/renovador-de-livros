<script lang="ts">
	let {
		label,
		checked = $bindable(false),
		disabled = false,
		description,
		onchange
	}: { label: string; checked?: boolean; disabled?: boolean; description?: string; onchange?: (v: boolean) => void } =
		$props();
	const uid = $props.id();
</script>

<div class="toggle">
	<button
		type="button"
		role="switch"
		class="toggle__switch"
		id={`tg-${uid}`}
		aria-checked={checked}
		aria-labelledby={`tg-${uid}-l`}
		aria-describedby={description ? `tg-${uid}-d` : undefined}
		{disabled}
		onclick={() => {
			checked = !checked;
			onchange?.(checked);
		}}
	>
		<span class="toggle__thumb" aria-hidden="true"></span>
		<span class="toggle__state" aria-hidden="true">{checked ? 'SIM' : 'NÃO'}</span>
	</button>
	<span class="toggle__text">
		<span id={`tg-${uid}-l`} class="toggle__label">{label}</span>
		{#if description}<span id={`tg-${uid}-d`} class="toggle__desc">{description}</span>{/if}
	</span>
</div>

<style>
	.toggle {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.toggle__switch {
		position: relative;
		flex: none;
		display: inline-flex;
		align-items: center;
		width: 5.5rem;
		min-height: var(--target-min);
		padding: 0;
		border: var(--border);
		background: var(--color-paper-sunken);
		color: var(--color-ink);
		box-shadow: var(--shadow-hard);
		cursor: pointer;
	}
	.toggle__thumb {
		position: absolute;
		top: 0.25rem;
		bottom: 0.25rem;
		left: 0.25rem;
		width: 2rem;
		background: var(--color-ink);
		transition: left var(--duration-base) var(--ease-out);
	}
	.toggle__state {
		margin-left: auto;
		padding-right: var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 700;
	}
	.toggle__switch[aria-checked='true'] {
		background: var(--color-accent);
		color: var(--color-accent-ink);
	}
	.toggle__switch[aria-checked='true'] .toggle__thumb {
		left: calc(100% - 2.25rem);
		background: var(--color-accent-ink);
	}
	.toggle__switch[aria-checked='true'] .toggle__state {
		margin-left: 0;
		padding-left: var(--space-2);
	}
	.toggle__switch:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.toggle__text {
		display: grid;
	}
	.toggle__label {
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
	}
	.toggle__desc {
		color: var(--color-ink-muted);
		font-size: var(--text-sm);
	}
</style>
