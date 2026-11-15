<script lang="ts">
	import { onMount, tick } from 'svelte';
	import t from '#lib/i18n/pt-BR/estatisticas.ts';
	import { commands, type MonthlyStat, type StatsOverview } from '#lib/ipc/bindings.ts';
	import Select from '#lib/components/Select.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import DataTable, { type Column } from '#lib/components/DataTable.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import Button from '#lib/components/Button.svelte';
	import { reveal } from '#lib/motion/index.ts';
	import MonthlyChart from '#lib/features/estatisticas/MonthlyChart.svelte';
	import {
		fillMonths,
		formatInt,
		formatPercent,
		monthLabel,
		monthRate,
		rangeFor,
		type PeriodKey
	} from '#lib/features/estatisticas/stats.ts';

	const periods = (Object.keys(t.periods) as PeriodKey[]).map((value) => ({ value, label: t.periods[value] }));
	let period: PeriodKey = $state('12m');
	let status: 'loading' | 'ready' | 'error' = $state('loading');
	let overview: StatsOverview | null = $state(null);
	let monthly: MonthlyStat[] = $state([]);
	let tiles: HTMLElement | undefined = $state();
	let seq = 0;

	async function load(p: PeriodKey) {
		const mine = ++seq;
		status = 'loading';
		try {
			const range = rangeFor(p);
			const [ov, mo] = await Promise.all([commands.statsOverview(range), commands.statsMonthly(range)]);
			if (mine !== seq) return;
			overview = ov;
			monthly = fillMonths(mo);
			status = 'ready';
			await tick();
			if (tiles) reveal([...tiles.children]);
		} catch {
			if (mine === seq) status = 'error';
		}
	}

	let mounted = false;
	onMount(() => {
		mounted = true;
		load(period);
	});
	function onPeriod() {
		if (mounted) load(period);
	}

	type Row = { month: string; label: string; renewals: number; successes: number; failures: number; rate: string };
	const columns: Column<Row>[] = [
		{ key: 'label', label: t.cols.month },
		{ key: 'renewals', label: t.cols.renewals, numeric: true, sortable: true, mono: true },
		{ key: 'successes', label: t.cols.successes, numeric: true, sortable: true, mono: true },
		{ key: 'failures', label: t.cols.failures, numeric: true, sortable: true, mono: true },
		{ key: 'rate', label: t.cols.rate, numeric: true, mono: true }
	];
	const rows: Row[] = $derived(
		monthly.map((s) => ({
			month: s.month,
			label: monthLabel(s.month),
			renewals: s.renewals,
			successes: s.successes,
			failures: s.failures,
			rate: formatPercent(monthRate(s)) ?? t.noRate
		}))
	);
</script>

<svelte:head><title>{t.title} · Renovador de Livros</title></svelte:head>

<h1>{t.title}</h1>
<p class="intro">{t.intro}</p>

<div class="filters">
	<Select label={t.period} options={periods} bind:value={period} onchange={onPeriod} />
</div>

{#if status === 'loading'}
	<Skeleton lines={5} label={t.loading} height="2.5rem" />
{:else if status === 'error'}
	<EmptyState title={t.errorTitle}>
		{#snippet actions()}
			<Button variant="primary" onclick={() => load(period)}>{t.retry}</Button>
		{/snippet}
	</EmptyState>
{:else if overview}
	<section aria-label={t.title}>
		<dl class="tiles" bind:this={tiles}>
			<div class="tile"><dt>{t.tiles.distinctBooks}</dt><dd>{formatInt(overview.distinct_books)}</dd></div>
			<div class="tile"><dt>{t.tiles.activeLoans}</dt><dd>{formatInt(overview.active_loans)}</dd></div>
			<div class="tile"><dt>{t.tiles.renewals}</dt><dd>{formatInt(overview.renewals_total)}</dd></div>
			<div class="tile">
				<dt>{t.tiles.successRate}</dt>
				<dd>{formatPercent(overview.success_rate) ?? t.noRate}</dd>
			</div>
			<div class="tile"><dt>{t.tiles.daysGained}</dt><dd>{formatInt(overview.days_gained)}</dd></div>
		</dl>
	</section>

	<Ficha title={t.mostRenewed}>
		{#if overview.most_renewed}
			<p class="book">{overview.most_renewed.title}</p>
			{#if overview.most_renewed.author}<p class="muted">{overview.most_renewed.author}</p>{/if}
		{:else}
			<p class="muted">{t.noMostRenewed}</p>
		{/if}
	</Ficha>

	<section class="monthly" aria-labelledby="est-mensal">
		<h2 id="est-mensal">{t.monthlyTitle}</h2>
		{#if overview.renewals_total === 0 && monthly.every((m) => m.renewals === 0)}
			<EmptyState title={t.emptyTitle} description={t.emptyDescription} headingLevel={3} />
		{:else}
			<MonthlyChart data={monthly} />
			<DataTable caption={t.tableCaption} {columns} {rows} rowKey="month" />
		{/if}
	</section>
{/if}

<style>
	.intro {
		color: var(--color-ink-muted);
		margin-block-end: var(--space-4);
	}
	.filters {
		max-width: 20rem;
		margin-block-end: var(--space-4);
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
		gap: var(--space-3);
		margin: 0 0 var(--space-4);
	}
	.tile {
		border: var(--border);
		box-shadow: var(--shadow-hard);
		background: var(--color-paper-raised);
		padding: var(--space-3);
		display: flex;
		flex-direction: column-reverse;
		gap: var(--space-1);
	}
	.tile dt {
		font-family: var(--font-display);
		font-size: var(--text-xs);
		text-transform: uppercase;
		letter-spacing: var(--tracking-caps);
		color: var(--color-ink-muted);
	}
	.tile dd {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xl);
		font-weight: var(--weight-bold);
		color: var(--color-ink);
		font-variant-numeric: tabular-nums;
	}
	.book {
		font-family: var(--font-body);
		font-size: var(--text-lg);
		font-weight: var(--weight-bold);
		margin: 0;
	}
	.muted {
		color: var(--color-ink-muted);
		margin: 0;
	}
	.monthly {
		margin-block-start: var(--space-5);
		display: grid;
		gap: var(--space-4);
	}
</style>
