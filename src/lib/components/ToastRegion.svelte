<script lang="ts">
	import { toasts, dismissToast } from './toast.svelte.ts';

	const prefix = { info: 'Aviso', ok: 'Sucesso', warn: 'Atenção', danger: 'Erro' } as const;
</script>

<!-- A região existe sempre no DOM para que leitores de tela anunciem as inserções. -->
<div class="toasts" role="status" aria-live="polite" aria-relevant="additions text">
	{#each toasts as t (t.id)}
		<div class={['toast', `toast--${t.kind}`]}>
			<p class="toast__msg"><strong>{prefix[t.kind]}:</strong> {t.message}</p>
			<button type="button" class="toast__x" aria-label="Dispensar aviso" onclick={() => dismissToast(t.id)}>
				<span aria-hidden="true">✕</span>
			</button>
		</div>
	{/each}
</div>

<style>
	.toasts {
		position: fixed;
		right: var(--space-3);
		bottom: var(--space-3);
		z-index: 50;
		display: grid;
		gap: var(--space-2);
		max-width: min(24rem, calc(100vw - 2rem));
	}
	.toast {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
		border: var(--border);
		border-left-width: 0.75rem;
		background: var(--color-paper-raised);
		color: var(--color-ink);
		box-shadow: var(--shadow-hard);
	}
	.toast--ok {
		border-left-color: var(--color-ok);
	}
	.toast--warn {
		border-left-color: var(--color-warn);
	}
	.toast--danger {
		border-left-color: var(--color-danger);
	}
	.toast--info {
		border-left-color: var(--color-info);
	}
	.toast__msg {
		margin: 0;
		flex: 1;
	}
	.toast__x {
		flex: none;
		width: var(--target-min);
		height: var(--target-min);
		border: var(--border);
		background: var(--color-paper);
		color: var(--color-ink);
		cursor: pointer;
	}
</style>
