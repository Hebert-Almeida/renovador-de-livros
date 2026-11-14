<script lang="ts" module>
	export type StepState = 'pending' | 'active' | 'done' | 'error';
	export interface Step {
		id: string;
		label: string;
		state: StepState;
	}
</script>

<script lang="ts">
	let { steps, label = 'Progresso' }: { steps: Step[]; label?: string } = $props();
	const stateText = { pending: 'pendente', active: 'em andamento', done: 'concluído', error: 'falhou' } as const;
	const mark = { pending: '', active: '…', done: '✓', error: '✕' } as const;
	const current = $derived(steps.find((s) => s.state === 'active' || s.state === 'error'));
	const doneCount = $derived(steps.filter((s) => s.state === 'done').length);
</script>

<div class="steps">
	<ol class="steps__list" aria-label={label}>
		{#each steps as s, i (s.id)}
			<li class={['step', `step--${s.state}`]} aria-current={s.state === 'active' ? 'step' : undefined}>
				<span class="step__num" aria-hidden="true">{mark[s.state] || i + 1}</span>
				<span class="step__label">{s.label}<span class="visually-hidden">, {stateText[s.state]}</span></span>
			</li>
		{/each}
	</ol>
	<p class="visually-hidden" aria-live="polite">
		{doneCount} de {steps.length} etapas concluídas{current ? `. ${current.label}: ${stateText[current.state]}` : ''}
	</p>
</div>

<style>
	.steps__list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.step {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-3) var(--space-1) var(--space-1);
		border: var(--border);
		background: var(--color-paper-raised);
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
	}
	.step__num {
		display: inline-grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		border: var(--border-width-thin) solid var(--color-rule);
		font-family: var(--font-mono);
	}
	.step--pending {
		color: var(--color-ink-muted);
		border-style: dashed;
	}
	.step--active {
		box-shadow: var(--shadow-hard);
		background: var(--color-accent);
		color: var(--color-accent-ink);
	}
	.step--active .step__num {
		border-color: currentColor;
	}
	.step--done .step__num {
		background: var(--color-ink);
		color: var(--color-paper);
	}
	.step--error {
		color: var(--color-danger);
		background: var(--color-danger-bg);
	}
</style>
