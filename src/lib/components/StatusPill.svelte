<script lang="ts" module>
	export type Status = 'ok' | 'warn' | 'danger' | 'info' | 'neutral';
</script>

<script lang="ts">
	let { status = 'neutral', label }: { status?: Status; label: string } = $props();
	// Cor nunca é o único indicador: cada estado tem também um símbolo.
	const icon = { ok: '✓', warn: '!', danger: '✕', info: 'i', neutral: '•' } as const;
</script>

<span class={['pill', `pill--${status}`]}>
	<span class="pill__icon" aria-hidden="true">{icon[status]}</span>
	{label}
</span>

<style>
	.pill {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: 0.125rem var(--space-2) 0.125rem 0.125rem;
		border: var(--border-width-thin) solid currentColor;
		border-radius: var(--radius-pill);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		font-family: var(--font-display);
		font-size: var(--text-sm);
		font-weight: var(--weight-bold);
		white-space: nowrap;
	}
	.pill__icon {
		display: inline-grid;
		place-items: center;
		width: 1.25rem;
		height: 1.25rem;
		border-radius: var(--radius-pill);
		background: currentColor;
		font-size: var(--text-xs);
	}
	.pill__icon {
		color: var(--color-paper-raised);
		background: var(--pill-c, var(--color-ink));
	}
	.pill--ok {
		color: var(--color-ok);
		background: var(--color-ok-bg);
		--pill-c: var(--color-ok);
	}
	.pill--warn {
		color: var(--color-warn);
		background: var(--color-warn-bg);
		--pill-c: var(--color-warn);
	}
	.pill--danger {
		color: var(--color-danger);
		background: var(--color-danger-bg);
		--pill-c: var(--color-danger);
	}
	.pill--info {
		color: var(--color-info);
		background: var(--color-info-bg);
		--pill-c: var(--color-info);
	}
	.pill--ok .pill__icon,
	.pill--warn .pill__icon,
	.pill--danger .pill__icon,
	.pill--info .pill__icon {
		color: var(--color-paper-raised);
	}
</style>
