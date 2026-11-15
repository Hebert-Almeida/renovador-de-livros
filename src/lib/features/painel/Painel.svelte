<script lang="ts">
	import { onMount, tick } from 'svelte';
	import Button from '#lib/components/Button.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import Stamp from '#lib/components/Stamp.svelte';
	import Select from '#lib/components/Select.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import StatusPill from '#lib/components/StatusPill.svelte';
	import ProgressSteps from '#lib/components/ProgressSteps.svelte';
	import HeroBook from '#lib/three/HeroBook.svelte';
	import { reveal } from '#lib/motion/index.ts';
	import t from '#lib/i18n/pt-BR/painel.ts';
	import { PainelState } from './painel.svelte.ts';
	import { dueLabel, dueTone, earliest, formatDate, formatDateTime, latest, quotaView } from './format.ts';

	const MAX_DUE = 5;
	const s = new PainelState();
	let now = $state(new Date());
	let grid: HTMLElement | undefined = $state();
	let revealed = false;

	const upcoming = $derived(s.loans.slice(0, MAX_DUE));
	const automation = $derived(s.status?.automation);
	const lastRenewal = $derived(latest(s.accounts.map((a) => a.last_renewal_at)));
	const nextRenewal = $derived(earliest(s.accounts.map((a) => a.next_renewal_at)));
	const quota = $derived(s.selectedAccount === null ? undefined : s.quotas[s.selectedAccount]);
	const qv = $derived(quotaView(quota, now));
	const accountOptions = $derived(
		s.accounts.map((a) => ({ value: String(a.id), label: `${a.library_name} · ${a.login}` }))
	);
	const pill = $derived.by((): { status: 'ok' | 'warn' | 'neutral'; label: string } => {
		if (!automation?.enabled) return { status: 'neutral', label: t.automationOff };
		if (automation.paused_reason || !automation.remote_enabled) return { status: 'warn', label: t.automationPaused };
		return { status: 'ok', label: t.automationOn };
	});

	let accountValue = $state('');
	$effect(() => {
		if (s.selectedAccount !== null) accountValue = String(s.selectedAccount);
	});
	function onAccountChange() {
		const id = Number(accountValue);
		if (accountValue && id !== s.selectedAccount) s.selectedAccount = id;
	}

	$effect(() => {
		if (grid && !revealed) {
			revealed = true;
			reveal([...grid.querySelectorAll('.painel__card')]);
		}
	});

	// O botão fica desabilitado durante o ciclo e perde o foco; devolve-o ao terminar.
	async function runSync() {
		await s.sync();
		await tick();
		document.querySelector<HTMLButtonElement>('[data-sync-now]:not(:disabled)')?.focus();
	}

	onMount(() => {
		const off = s.subscribe();
		void s.load();
		const timer = setInterval(() => (now = new Date()), 30_000);
		return () => {
			off();
			clearInterval(timer);
		};
	});
</script>

<svelte:head><title>{t.title} · Renovador de Livros</title></svelte:head>

<h1>{t.title}</h1>
<p class="painel__lead">{t.lead}</p>

{#if s.loading}
	<Skeleton lines={4} label={t.loading} />
{:else if s.loadError}
	<div class="painel__error" role="alert">
		<p>{t.loadError} {s.loadError}</p>
		<Button onclick={() => s.load()}>{t.retry}</Button>
	</div>
{:else if s.accounts.length === 0}
	<EmptyState title={t.noAccountTitle} description={t.noAccountDesc}>
		{#snippet illustration()}<HeroBook class="painel__hero" />{/snippet}
		{#snippet actions()}<a class="painel__cta" href="/contas">{t.addAccount}</a>{/snippet}
	</EmptyState>
{:else}
	<div class="painel" bind:this={grid}>
		<section class="painel__card painel__due" aria-labelledby="p-due">
			<h2 id="p-due">{t.dueHeading}</h2>
			{#if upcoming.length === 0}
				<EmptyState title={t.emptyTitle} description={t.emptyDesc} headingLevel={3}>
					{#snippet illustration()}<HeroBook class="painel__hero" />{/snippet}
				</EmptyState>
			{:else}
				<ol class="painel__list" aria-label={t.dueTable}>
					{#each upcoming as v (v.loan.id)}
						<li class="painel__loan">
							<div class="painel__book">
								<a class="painel__book-title" href={`/emprestimos/${v.loan.id}`}>{v.book.title}</a>
								{#if v.book.author}<span class="painel__author">{v.book.author}</span>{/if}
							</div>
							<span class="painel__date">
								{t.colDue}: <time datetime={v.loan.due_date}>{formatDate(v.loan.due_date)}</time>
							</span>
							<StatusPill status={dueTone(v.loan.due_date, now)} label={dueLabel(v.loan.due_date, now)} />
							<span class="painel__meta">
								{t.colRenewals}: {v.loan.renewals_count}{v.loan.renewals_limit != null ? `/${v.loan.renewals_limit}` : ''}
								· {t.colAuto}: {v.loan.auto_renew ? t.autoOn : t.autoOff}
							</span>
						</li>
					{/each}
				</ol>
				<a class="painel__more" href="/emprestimos">{t.seeAll}</a>
			{/if}
		</section>

		<div class="painel__card">
			<Ficha title={t.automationHeading} subtitle={automation?.paused_reason ?? undefined}>
				<p><StatusPill status={pill.status} label={pill.label} /></p>
				{#if automation && !automation.remote_enabled}<p>{t.remoteOff}</p>{/if}
				<dl class="painel__facts">
					<dt>{t.lastRenewal}</dt>
					<dd>
						{#if lastRenewal}<time datetime={lastRenewal}>{formatDateTime(lastRenewal)}</time>{:else}{t.never}{/if}
					</dd>
					<dt>{t.nextRenewal}</dt>
					<dd>
						{#if nextRenewal}<time datetime={nextRenewal}>{formatDateTime(nextRenewal)}</time>{:else}{t.none}{/if}
					</dd>
					<dt>{t.nextRun}</dt>
					<dd>
						{#if s.status?.next_run_at}<time datetime={s.status.next_run_at}>{formatDateTime(s.status.next_run_at)}</time
							>{:else}{t.none}{/if}
					</dd>
				</dl>
				{#snippet stamps()}
					{#if lastRenewal}<Stamp date={lastRenewal.slice(0, 10)} />{/if}
				{/snippet}
			</Ficha>
		</div>

		<div class="painel__card">
			<Ficha title={t.syncHeading}>
				{#if s.accounts.length > 1}
					<Select
						label={t.syncAccount}
						options={accountOptions}
						bind:value={accountValue}
						onchange={onAccountChange}
						disabled={s.syncing}
					/>
				{/if}
				<p class="painel__quota" id="p-quota">{qv.text}</p>
				{#if s.syncing || s.progress}
					<ProgressSteps label={t.progressLabel} steps={s.steps} />
				{/if}
				<p class="painel__live" role="status" aria-live="polite" aria-atomic="true">{s.announcement}</p>
				{#snippet actions()}
					<Button
						variant="primary"
						loading={s.syncing}
						disabled={!qv.canSync || s.syncing}
						aria-describedby="p-quota"
						data-sync-now
						onclick={runSync}
					>
						{s.syncing ? t.syncing : t.syncButton}
					</Button>
				{/snippet}
			</Ficha>
		</div>
	</div>
{/if}

<style>
	.painel__lead {
		margin: 0 0 var(--space-4);
		color: var(--color-ink-muted);
	}
	.painel {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
		gap: var(--space-4);
		align-items: start;
	}
	.painel__due {
		grid-column: 1 / -1;
		display: grid;
		gap: var(--space-3);
	}
	.painel__due h2 {
		margin: 0;
	}
	.painel__list {
		display: grid;
		margin: 0;
		padding: 0;
		list-style: none;
		border: var(--border);
		background: var(--color-paper-raised);
		box-shadow: var(--shadow-hard);
	}
	.painel__loan {
		display: grid;
		grid-template-columns: minmax(0, 2fr) auto auto;
		gap: var(--space-1) var(--space-3);
		align-items: center;
		padding: var(--space-3);
		border-bottom: var(--border-width-thin) solid var(--color-rule);
	}
	.painel__loan:last-child {
		border-bottom: 0;
	}
	.painel__book {
		display: grid;
	}
	.painel__book-title {
		display: inline-flex;
		align-items: center;
		min-height: var(--target-min);
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
	}
	.painel__author,
	.painel__meta {
		color: var(--color-ink-muted);
		font-size: var(--text-sm);
	}
	.painel__meta {
		grid-column: 1 / -1;
	}
	.painel__date {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
	}
	.painel__more,
	.painel__cta {
		display: inline-flex;
		align-items: center;
		min-height: var(--target-min);
		justify-self: start;
	}
	.painel__cta {
		padding: 0 var(--space-3);
		border: var(--border);
		background: var(--color-accent);
		color: var(--color-accent-ink);
		box-shadow: var(--shadow-hard);
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		text-decoration: none;
	}
	.painel__facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: var(--space-1) var(--space-3);
		margin: 0;
	}
	.painel__facts dt {
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
	}
	.painel__facts dd {
		margin: 0;
		font-family: var(--font-mono);
	}
	.painel__quota {
		margin: 0;
	}
	.painel__live {
		margin: 0;
		min-height: 1.5em;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
	}
	.painel__error {
		display: grid;
		gap: var(--space-3);
		justify-items: start;
	}
	:global(.painel__hero) {
		width: 12rem;
		height: 9rem;
	}
	@media (max-width: 40rem) {
		.painel__loan {
			grid-template-columns: 1fr;
		}
	}
</style>
