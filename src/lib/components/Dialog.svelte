<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		open = $bindable(false),
		title,
		children,
		actions,
		onclose
	}: { open?: boolean; title: string; children?: Snippet; actions?: Snippet; onclose?: () => void } = $props();

	const uid = $props.id();
	let dialog: HTMLDialogElement | undefined = $state();
	let opener: Element | null = null;

	$effect(() => {
		const d = dialog;
		if (!d) return;
		if (open && !d.open) {
			opener = document.activeElement;
			if (typeof d.showModal === 'function') d.showModal();
			else d.setAttribute('open', '');
		} else if (!open && d.open) {
			if (typeof d.close === 'function') d.close();
			else d.removeAttribute('open');
		}
	});

	function handleClose() {
		open = false;
		onclose?.();
		if (opener instanceof HTMLElement) opener.focus();
		opener = null;
	}
</script>

<dialog
	bind:this={dialog}
	class="dialog"
	aria-labelledby={`dlg-${uid}`}
	onclose={handleClose}
	onclick={(e) => {
		if (e.target === dialog) open = false;
	}}
>
	<div class="dialog__panel">
		<header class="dialog__head">
			<h2 id={`dlg-${uid}`} class="dialog__title">{title}</h2>
			<button type="button" class="dialog__x" aria-label="Fechar" onclick={() => (open = false)}>
				<span aria-hidden="true">✕</span>
			</button>
		</header>
		{#if children}<div class="dialog__body">{@render children()}</div>{/if}
		{#if actions}<footer class="dialog__actions">{@render actions()}</footer>{/if}
	</div>
</dialog>

<style>
	.dialog {
		padding: 0;
		border: var(--border);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		box-shadow: var(--shadow-hard-lg);
		max-width: min(36rem, calc(100vw - 2rem));
		width: 100%;
	}
	.dialog::backdrop {
		background: var(--color-backdrop);
	}
	.dialog__panel {
		padding: var(--space-4);
	}
	.dialog__head {
		display: flex;
		align-items: start;
		justify-content: space-between;
		gap: var(--space-3);
		border-bottom: var(--border);
		padding-bottom: var(--space-2);
		margin-bottom: var(--space-3);
	}
	.dialog__title {
		margin: 0;
		font-size: var(--text-lg);
	}
	.dialog__x {
		width: var(--target-min);
		height: var(--target-min);
		flex: none;
		border: var(--border);
		background: var(--color-paper);
		color: var(--color-ink);
		font-size: var(--text-lg);
		cursor: pointer;
	}
	.dialog__actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-3);
		margin-top: var(--space-4);
	}
</style>
