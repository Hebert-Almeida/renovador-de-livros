<script lang="ts">
	import { page } from '$app/state';
	import { commands, type LoanDetail_Serialize } from '#lib/ipc/index.ts';
	import Ficha from '#lib/components/Ficha.svelte';
	import Stamp from '#lib/components/Stamp.svelte';
	import Toggle from '#lib/components/Toggle.svelte';
	import Button from '#lib/components/Button.svelte';
	import Dialog from '#lib/components/Dialog.svelte';
	import StatusPill from '#lib/components/StatusPill.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import { toast } from '#lib/components/toast.svelte.ts';
	import Cover from '#lib/features/emprestimos/Cover.svelte';
	import { formatDate, loanPill, eventLabel, sortEvents, errorMessage } from '#lib/features/emprestimos/loans.ts';
	import t from '#lib/i18n/pt-BR/emprestimos.ts';

	const id = $derived(Number(page.params.id));
	let detail = $state<LoanDetail_Serialize | null>(null);
	let loading = $state(true);
	let error: string | null = $state(null);
	let autoRenew = $state(false);
	let busy = $state(false);
	let renewing = $state(false);
	let confirmOpen = $state(false);
	/** Ids de carimbos já exibidos: só os novos são animados. */
	const seen = new Set<number>();
	let firstLoad = true;

	async function load(loanId: number) {
		try {
			const d = await commands.loanGet(loanId);
			if (loanId !== id) return;
			if (firstLoad) {
				seen.clear();
				for (const e of d.events) seen.add(e.id);
			}
			firstLoad = false;
			error = null;
			detail = d;
			autoRenew = d.view.loan.auto_renew;
		} catch (e) {
			if (loanId !== id) return;
			if (detail) {
				toast(errorMessage(e), 'danger');
			} else {
				const kind = e && typeof e === 'object' && 'kind' in e ? (e as { kind: string }).kind : '';
				error = kind === 'not_found' ? t.notFound : errorMessage(e);
			}
		} finally {
			if (loanId === id) loading = false;
		}
	}

	$effect(() => {
		const loanId = id;
		loading = true;
		firstLoad = true;
		detail = null;
		error = null;
		if (!Number.isInteger(loanId)) {
			error = t.notFound;
			loading = false;
			return;
		}
		void load(loanId);
	});

	const events = $derived(detail ? sortEvents(detail.events) : []);
	const returned = $derived(detail?.view.loan.status === 'returned');
	const pill = $derived(detail ? loanPill(detail.view) : null);

	async function setAuto(on: boolean) {
		if (!detail) return;
		busy = true;
		try {
			await commands.loanSetAutoRenew(detail.view.loan.id, on);
			detail.view.loan.auto_renew = on;
			toast(on ? t.autoRenewOn : t.autoRenewOff, 'ok');
		} catch (e) {
			autoRenew = !on;
			toast(errorMessage(e), 'danger');
		} finally {
			busy = false;
		}
	}

	async function renewNow() {
		if (!detail) return;
		const loanId = detail.view.loan.id;
		renewing = true;
		try {
			// Renova só este livro, mesmo fora da janela de vencimento (mesmo gate e cota).
			const r = await commands.loanRenew(loanId);
			const ok = r.renewed.some((e) => e.loan_id === loanId);
			toast(t.renewDone(ok ? 1 : 0), ok ? 'ok' : 'info');
		} catch (e) {
			toast(errorMessage(e), 'danger');
		} finally {
			renewing = false;
		}
		await load(loanId);
	}

	async function markReturned() {
		if (!detail) return;
		const loanId = detail.view.loan.id;
		busy = true;
		try {
			await commands.loanMarkReturned(loanId);
			confirmOpen = false;
			toast(t.returnedDone, 'ok');
		} catch (e) {
			confirmOpen = false;
			toast(errorMessage(e), 'danger');
		} finally {
			busy = false;
		}
		await load(loanId);
	}
</script>

<svelte:head>
	<title>{detail ? detail.view.book.title : t.title} · Renovador de Livros</title>
</svelte:head>

<p class="back"><a href="/emprestimos"><span aria-hidden="true">←</span> {t.back}</a></p>

{#if loading}
	<h1>{t.title}</h1>
	<Skeleton lines={4} />
{:else if error || !detail}
	<h1>{t.title}</h1>
	<p role="alert">{error ?? t.notFound}</p>
{:else}
	{@const v = detail.view}
	<h1 class="visually-hidden">{v.book.title}</h1>
	<div class="sheet">
		<Cover title={v.book.title} author={v.book.author} />
		<div class="sheet__ficha">
			<Ficha
				title={v.book.title}
				subtitle={[v.book.author, v.book.call_number && `${t.callNumber} ${v.book.call_number}`].filter(Boolean).join(' · ') ||
					undefined}
			>
				{#if pill}<StatusPill status={pill.status} label={pill.label} />{/if}
				<dl class="facts">
					<div><dt>{t.due}</dt><dd><time datetime={v.loan.due_date}>{formatDate(v.loan.due_date)}</time></dd></div>
					{#if v.loan.loaned_at}
						<div><dt>{t.loanedAt}</dt><dd><time datetime={v.loan.loaned_at}>{formatDate(v.loan.loaned_at)}</time></dd></div>
					{/if}
					{#if v.loan.returned_at}
						<div>
							<dt>{t.returnedAt}</dt><dd><time datetime={v.loan.returned_at}>{formatDate(v.loan.returned_at)}</time></dd>
						</div>
					{/if}
					<div><dt>{t.renewals}</dt><dd>{t.renewalsOf(v.loan.renewals_count, v.loan.renewals_limit)}</dd></div>
					{#if v.book.library_branch}
						<div><dt>{t.branch}</dt><dd>{v.book.library_branch}</dd></div>
					{/if}
				</dl>
				{#if !returned}
					<Toggle
						label={t.autoRenew}
						description={t.autoRenewDesc}
						bind:checked={autoRenew}
						disabled={busy}
						onchange={(on) => void setAuto(on)}
					/>
				{/if}

				{#snippet stamps()}
					{#each events as e (e.id)}
						<Stamp date={e.attempted_at.slice(0, 10)} label={eventLabel(e)} animate={!seen.has(e.id)} />
					{/each}
				{/snippet}

				{#snippet actions()}
					{#if !returned}
						<Button variant="primary" loading={renewing} disabled={busy} onclick={renewNow}>
							{renewing ? t.renewing : t.renewNow}
						</Button>
						<Button variant="ghost" disabled={busy || renewing} onclick={() => (confirmOpen = true)}>{t.markReturned}</Button>
					{/if}
				{/snippet}
			</Ficha>
		</div>
	</div>

	<section class="history" aria-labelledby="carimbos">
		<h2 id="carimbos">{t.stamps}</h2>
		{#if events.length === 0}
			<p>{t.stampsEmpty}</p>
		{:else}
			<ol class="log">
				{#each events as e (e.id)}
					<li>
						<time class="mono" datetime={e.attempted_at}>{formatDate(e.attempted_at)}</time>
						<strong>{eventLabel(e)}</strong>
						<span>({t.trigger[e.trigger] ?? e.trigger})</span>
						{#if e.new_due_date}<span>· {t.due}: {formatDate(e.new_due_date)}</span>{/if}
						{#if e.message}<span class="muted">· {e.message}</span>{/if}
					</li>
				{/each}
			</ol>
		{/if}
	</section>

	<Dialog bind:open={confirmOpen} title={t.confirmReturnTitle}>
		<p>{t.confirmReturnBody}</p>
		{#snippet actions()}
			<Button onclick={() => (confirmOpen = false)}>{t.cancel}</Button>
			<Button variant="danger" loading={busy} onclick={markReturned}>{t.confirm}</Button>
		{/snippet}
	</Dialog>
{/if}

<style>
	.back {
		margin-block-end: var(--space-4);
	}
	.back a {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		min-height: var(--target-min);
		color: var(--color-ink);
		font-weight: var(--weight-bold);
	}
	.sheet {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-5);
		align-items: flex-start;
	}
	.sheet__ficha {
		flex: 1 1 22rem;
		min-width: 0;
	}
	.facts {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
		gap: var(--space-3);
		margin: var(--space-3) 0;
	}
	.facts dt {
		font-size: var(--text-xs);
		text-transform: uppercase;
		letter-spacing: var(--tracking-caps);
		color: var(--color-ink-muted);
	}
	.facts dd {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-lg);
	}
	.history {
		margin-block-start: var(--space-5);
	}
	.log {
		display: grid;
		gap: var(--space-2);
		margin: 0;
		padding-inline-start: var(--space-4);
		border-inline-start: var(--border);
		list-style: none;
	}
	.log li {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.mono {
		font-family: var(--font-mono);
	}
	.muted {
		color: var(--color-ink-muted);
	}
</style>
