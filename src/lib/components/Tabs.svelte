<script lang="ts" module>
	export interface TabItem {
		id: string;
		label: string;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		tabs,
		label,
		selected = $bindable(tabs[0]?.id ?? ''),
		panel
	}: { tabs: TabItem[]; label: string; selected?: string; panel: Snippet<[string]> } = $props();

	const uid = $props.id();
	let buttons: HTMLButtonElement[] = $state([]);

	function onkeydown(e: KeyboardEvent, i: number) {
		let next = -1;
		if (e.key === 'ArrowRight') next = (i + 1) % tabs.length;
		else if (e.key === 'ArrowLeft') next = (i - 1 + tabs.length) % tabs.length;
		else if (e.key === 'Home') next = 0;
		else if (e.key === 'End') next = tabs.length - 1;
		if (next < 0) return;
		e.preventDefault();
		selected = tabs[next].id;
		buttons[next]?.focus();
	}
</script>

<div class="tabs">
	<div role="tablist" aria-label={label} class="tabs__list">
		{#each tabs as t, i (t.id)}
			<button
				bind:this={buttons[i]}
				type="button"
				role="tab"
				class="tabs__tab"
				id={`tab-${uid}-${t.id}`}
				aria-selected={selected === t.id}
				aria-controls={`panel-${uid}-${t.id}`}
				tabindex={selected === t.id ? 0 : -1}
				onclick={() => (selected = t.id)}
				onkeydown={(e) => onkeydown(e, i)}>{t.label}</button
			>
		{/each}
	</div>
	{#each tabs as t (t.id)}
		<div
			role="tabpanel"
			class="tabs__panel"
			id={`panel-${uid}-${t.id}`}
			aria-labelledby={`tab-${uid}-${t.id}`}
			hidden={selected !== t.id}
			tabindex="0"
		>
			{#if selected === t.id}{@render panel(t.id)}{/if}
		</div>
	{/each}
</div>

<style>
	.tabs__list {
		display: flex;
		flex-wrap: wrap;
		gap: 0;
		border-bottom: var(--border);
	}
	.tabs__tab {
		min-height: var(--target-min);
		padding: var(--space-2) var(--space-4);
		margin-bottom: calc(-1 * var(--border-width));
		border: var(--border);
		border-right-width: 0;
		background: var(--color-paper-sunken);
		color: var(--color-ink);
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		cursor: pointer;
	}
	.tabs__tab:last-child {
		border-right-width: var(--border-width);
	}
	.tabs__tab[aria-selected='true'] {
		background: var(--color-paper-raised);
		border-bottom-color: var(--color-paper-raised);
		text-decoration: underline;
		text-decoration-thickness: 0.1875rem;
		text-underline-offset: 0.3em;
	}
	.tabs__panel {
		padding: var(--space-4);
		border: var(--border);
		border-top: 0;
		background: var(--color-paper-raised);
	}
</style>
