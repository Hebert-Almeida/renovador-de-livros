<script lang="ts">
	import { onMount } from 'svelte';
	import Button from '#lib/components/Button.svelte';
	import Select from '#lib/components/Select.svelte';
	import TextField from '#lib/components/TextField.svelte';
	import DataTable from '#lib/components/DataTable.svelte';
	import Dialog from '#lib/components/Dialog.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import { toast } from '#lib/components/toast.svelte.ts';
	import { reveal } from '#lib/motion/index.ts';
	import { commands } from '#lib/ipc/index.ts';
	import type { Account_Serialize, ExportFormat, RenewalEvent_Serialize } from '#lib/ipc/index.ts';
	import t from '#lib/i18n/pt-BR/historico.ts';
	import {
		PAGE_SIZE,
		RESULTS,
		emptyForm,
		toFilter,
		isRangeValid,
		formatDateTime,
		dueChange,
		startOfDay,
		exportHistory
	} from './history.ts';

	let accounts: Account_Serialize[] = $state([]);
	let form = $state(emptyForm());
	let applied = $state(emptyForm());
	let offset = $state(0);
	let items: RenewalEvent_Serialize[] = $state([]);
	let total = $state(0);
	let loading = $state(true);
	let failed = $state(false);
	let rangeError = $state('');
	let exporting: ExportFormat | null = $state(null);
	let clearOpen = $state(false);
	let clearBefore = $state('');
	let clearing = $state(false);
	let content: HTMLElement | undefined = $state();

	const filtered = $derived(Object.values(applied).some(Boolean));
	const accountOptions = $derived([
		{ value: '', label: t.allAccounts },
		...accounts.map((a) => ({ value: String(a.id), label: `${a.login} · ${a.library_name}` }))
	]);
	const resultOptions = [{ value: '', label: t.allResults }, ...RESULTS.map((r) => ({ value: r, label: t.results[r] }))];
	const rows = $derived(
		items.map((e) => ({
			id: e.id,
			attemptedAt: formatDateTime(e.attempted_at),
			loan: t.loanRef(e.loan_id),
			trigger: t.triggers[e.trigger],
			result: t.results[e.result],
			dueChange: dueChange(e),
			message: e.message ?? '—'
		}))
	);
	type Row = (typeof rows)[number];
	const columns: { key: keyof Row & string; label: string; sortable?: boolean; mono?: boolean }[] = [
		{ key: 'attemptedAt', label: t.columns.attemptedAt, mono: true },
		{ key: 'loan', label: t.columns.loan, sortable: true },
		{ key: 'trigger', label: t.columns.trigger, sortable: true },
		{ key: 'result', label: t.columns.result, sortable: true },
		{ key: 'dueChange', label: t.columns.dueChange, mono: true },
		{ key: 'message', label: t.columns.message }
	];

	let seq = 0;
	async function load() {
		const mine = ++seq;
		loading = true;
		failed = false;
		try {
			const page = await commands.historyList(toFilter(applied), { offset, limit: PAGE_SIZE });
			if (mine !== seq) return;
			items = page.items;
			total = page.total;
		} catch {
			if (mine !== seq) return;
			failed = true;
		} finally {
			if (mine === seq) loading = false;
		}
	}

	function apply(e: SubmitEvent) {
		e.preventDefault();
		if (!isRangeValid(form)) {
			rangeError = t.invalidRange;
			return;
		}
		rangeError = '';
		applied = { ...form };
		offset = 0;
		load();
	}

	function reset() {
		form = emptyForm();
		applied = emptyForm();
		rangeError = '';
		offset = 0;
		load();
	}

	function go(delta: number) {
		offset = Math.max(0, offset + delta * PAGE_SIZE);
		load();
	}

	async function doExport(format: ExportFormat) {
		exporting = format;
		try {
			const n = await exportHistory(format);
			if (n !== null) toast(t.exported(n), 'ok');
		} catch {
			toast(t.exportError, 'danger');
		} finally {
			exporting = null;
		}
	}

	async function doClear() {
		clearing = true;
		try {
			const n = await commands.historyClear(clearBefore ? startOfDay(clearBefore) : null);
			clearOpen = false;
			clearBefore = '';
			toast(t.cleared(n), 'ok');
			offset = 0;
			await load();
		} catch {
			toast(t.clearError, 'danger');
		} finally {
			clearing = false;
		}
	}

	onMount(() => {
		commands.accountList().then(
			(a) => (accounts = a),
			() => (accounts = [])
		);
		load().then(() => {
			if (content) reveal([...content.children]);
		});
	});
</script>

<div class="historico" bind:this={content}>
	<header class="head">
		<h1>{t.title}</h1>
		<p class="intro">{t.intro}</p>
	</header>

	<form class="filters" onsubmit={apply} aria-label={t.filters}>
		<Select label={t.account} options={accountOptions} bind:value={form.account} />
		<Select label={t.result} options={resultOptions} bind:value={form.result} />
		<TextField label={t.from} type="date" bind:value={form.from} />
		<TextField label={t.to} type="date" bind:value={form.to} error={rangeError || undefined} />
		<div class="filters__actions">
			<Button type="submit" variant="primary">{t.apply}</Button>
			<Button variant="ghost" onclick={reset}>{t.resetFilters}</Button>
		</div>
	</form>

	<div class="toolbar" role="group" aria-label={t.actions}>
		<Button loading={exporting === 'csv'} disabled={exporting !== null} onclick={() => doExport('csv')}>{t.exportCsv}</Button>
		<Button loading={exporting === 'json'} disabled={exporting !== null} onclick={() => doExport('json')}>{t.exportJson}</Button>
		<Button variant="danger" onclick={() => (clearOpen = true)}>{t.clear}</Button>
	</div>

	<div class="results" aria-busy={loading}>
		{#if loading && items.length === 0 && !failed}
			<Skeleton lines={5} label={t.loading} />
		{:else if failed}
			<EmptyState title={t.loadError} headingLevel={2}>
				{#snippet actions()}<Button onclick={load}>{t.retry}</Button>{/snippet}
			</EmptyState>
		{:else if items.length === 0}
			<EmptyState title={t.emptyTitle} description={filtered ? t.emptyFiltered : t.emptyDescription} headingLevel={2} />
		{:else}
			<DataTable caption={t.tableCaption} rowKey="id" {rows} {columns} />
			<nav class="pager" aria-label={t.pagination}>
				<Button variant="ghost" disabled={offset === 0 || loading} onclick={() => go(-1)}>{t.prev}</Button>
				<span aria-live="polite">{t.pageInfo(offset + 1, offset + items.length, total)}</span>
				<Button variant="ghost" disabled={offset + items.length >= total || loading} onclick={() => go(1)}>{t.next}</Button>
			</nav>
		{/if}
	</div>
</div>

<Dialog bind:open={clearOpen} title={t.clearTitle} onclose={() => (clearBefore = '')}>
	<p>{t.clearBody}</p>
	<TextField label={t.clearBefore} hint={t.clearBeforeHint} type="date" bind:value={clearBefore} />
	{#snippet actions()}
		<Button onclick={() => (clearOpen = false)}>{t.cancel}</Button>
		<Button variant="danger" loading={clearing} onclick={doClear}>{t.confirmClear}</Button>
	{/snippet}
</Dialog>

<style>
	.historico {
		display: grid;
		gap: var(--space-5);
	}
	.head {
		display: grid;
		gap: var(--space-2);
	}
	.head h1,
	.intro {
		margin: 0;
	}
	.filters {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: var(--space-4);
		align-items: end;
	}
	.filters__actions,
	.toolbar {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	.results {
		display: grid;
		gap: var(--space-3);
	}
	.pager {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
</style>
