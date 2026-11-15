<script lang="ts">
	import { tick } from 'svelte';
	import { commands, type LoanStatus, type LoanView_Serialize } from '#lib/ipc/index.ts';
	import TextField from '#lib/components/TextField.svelte';
	import Select from '#lib/components/Select.svelte';
	import Spine from '#lib/components/Spine.svelte';
	import StatusPill from '#lib/components/StatusPill.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import Button from '#lib/components/Button.svelte';
	import { reveal } from '#lib/motion/index.ts';
	import { formatDate, loanPill, errorMessage } from '#lib/features/emprestimos/loans.ts';
	import t from '#lib/i18n/pt-BR/emprestimos.ts';

	const statusOptions = [
		{ value: '', label: t.statusAll },
		{ value: 'active', label: t.statusActive },
		{ value: 'overdue', label: t.statusOverdue },
		{ value: 'returned', label: t.statusReturned }
	];

	let search = $state('');
	let status = $state('');
	let loans: LoanView_Serialize[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let shelf: HTMLElement | undefined = $state();
	let seq = 0;

	async function load(q: string, s: string) {
		const mine = ++seq;
		loading = true;
		error = null;
		try {
			const res = await commands.loanList({
				account_id: null,
				status: (s || null) as LoanStatus | null,
				search: q.trim() || null
			});
			if (mine !== seq) return;
			loans = res;
		} catch (e) {
			if (mine !== seq) return;
			error = errorMessage(e);
			loans = [];
		} finally {
			if (mine === seq) loading = false;
		}
		await tick();
		if (shelf) reveal([...shelf.querySelectorAll('.loan')]);
	}

	$effect(() => {
		const q = search;
		const s = status;
		const timer = setTimeout(() => void load(q, s), q ? 250 : 0);
		return () => clearTimeout(timer);
	});

	const filtered = $derived(search.trim() !== '' || status !== '');
</script>

<svelte:head><title>{t.title} · Renovador de Livros</title></svelte:head>

<h1>{t.title}</h1>
<p class="lead">{t.lead}</p>

<form class="filters" role="search" aria-label={t.filters} onsubmit={(e) => e.preventDefault()}>
	<TextField label={t.search} type="search" bind:value={search} autocomplete="off" />
	<Select label={t.status} options={statusOptions} bind:value={status} />
</form>

<p class="count" aria-live="polite">{loading ? t.loading : error ? '' : t.count(loans.length)}</p>

{#if loading && loans.length === 0}
	<Skeleton lines={3} label={t.loading} />
{:else if error}
	<div class="error" role="alert">
		<p>{error}</p>
		<Button onclick={() => load(search, status)}>{t.retry}</Button>
	</div>
{:else if loans.length === 0}
	{#if filtered}
		<EmptyState title={t.emptyFilteredTitle} description={t.emptyFilteredDescription}>
			{#snippet actions()}
				<Button
					onclick={() => {
						search = '';
						status = '';
					}}>{t.clearFilters}</Button
				>
			{/snippet}
		</EmptyState>
	{:else}
		<EmptyState title={t.emptyTitle} description={t.emptyDescription} />
	{/if}
{:else}
	<ul class="shelf" aria-label={t.shelf} bind:this={shelf}>
		{#each loans as v (v.loan.id)}
			{@const pill = loanPill(v)}
			<li class="loan">
				<Spine
					title={v.book.title}
					author={v.book.author ?? undefined}
					due={formatDate(v.loan.due_date)}
					href={`/emprestimos/${v.loan.id}`}
				/>
				<StatusPill status={pill.status} label={pill.label} />
			</li>
		{/each}
	</ul>
{/if}

<style>
	.lead {
		font-family: var(--font-body);
		color: var(--color-ink-muted);
		margin-block-end: var(--space-4);
	}
	.filters {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
		gap: var(--space-4);
		padding: var(--space-4);
		border: var(--border);
		background: var(--color-paper-raised);
		box-shadow: var(--shadow-hard);
		margin-block-end: var(--space-4);
	}
	.count {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		min-height: 1.5em;
	}
	.shelf {
		list-style: none;
		margin: 0;
		padding: var(--space-4) var(--space-4) 0;
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--space-4);
		border-block-end: calc(var(--border-width) * 3) solid var(--color-ink);
	}
	.loan {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
		padding-block-end: var(--space-3);
		max-width: 9rem;
		text-align: center;
	}
	.error {
		display: grid;
		gap: var(--space-3);
		justify-items: start;
		padding: var(--space-4);
		border: var(--border);
		background: var(--color-danger-bg);
	}
</style>
