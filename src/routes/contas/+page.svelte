<script lang="ts">
	import { onMount, tick } from 'svelte';
	import Button from '#lib/components/Button.svelte';
	import Dialog from '#lib/components/Dialog.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import StatusPill, { type Status } from '#lib/components/StatusPill.svelte';
	import Toggle from '#lib/components/Toggle.svelte';
	import { toast } from '#lib/components/toast.svelte.ts';
	import { reveal } from '#lib/motion/index.ts';
	import { commands, type Account_Serialize, type LibraryPreset, type QuotaStatus_Serialize } from '#lib/ipc/bindings.ts';
	import AccountForm, { type AccountFormData } from '#lib/features/contas/AccountForm.svelte';
	import { describeError, formatDateTime, MAX_ACCOUNTS } from '#lib/features/contas/errors.ts';
	import t from '#lib/i18n/pt-BR/contas.ts';

	let accounts = $state<Account_Serialize[]>([]);
	let presets = $state<LibraryPreset[]>([]);
	let loading = $state(true);
	let loadError = $state('');
	let list: HTMLElement | undefined = $state();

	let formOpen = $state(false);
	let formKey = $state(0);
	let editing = $state<Account_Serialize | undefined>();

	let testOpen = $state(false);
	let testing = $state<Account_Serialize | undefined>();
	let quota = $state<QuotaStatus_Serialize | null>(null);
	let quotaFailed = $state(false);
	let testBusy = $state(false);

	let removeOpen = $state(false);
	let removing = $state<Account_Serialize | undefined>();
	let removeAck = $state(false);
	let removeBusy = $state(false);

	const full = $derived(accounts.length >= MAX_ACCOUNTS);
	const statusKind: Record<Account_Serialize['status'], Status> = {
		active: 'ok',
		paused: 'neutral',
		invalid_password: 'danger',
		blocked: 'warn'
	};

	async function load() {
		loading = true;
		loadError = '';
		try {
			const [a, p] = await Promise.all([
				commands.accountList(),
				presets.length ? Promise.resolve(presets) : commands.libraryPresets()
			]);
			accounts = a;
			presets = p;
		} catch (e) {
			const msg = describeError(e);
			loadError = msg === t.errors.generic ? t.loadError : msg;
		} finally {
			loading = false;
		}
		await tick();
		if (list) reveal([...list.children]);
	}

	onMount(load);

	function openForm(a?: Account_Serialize) {
		if (!a && full) return;
		editing = a;
		formKey++;
		formOpen = true;
	}

	async function save(data: AccountFormData): Promise<boolean> {
		try {
			const v = await commands.libraryValidate(data.library_base_url);
			if (!v.ok) {
				toast(v.reason || t.invalidUrl, 'danger');
				return false;
			}
			const url = v.normalized_url || data.library_base_url;
			if (editing) {
				await commands.accountUpdate(editing.id, {
					library_base_url: url,
					library_name: data.library_name,
					login: data.login,
					password: data.password,
					clear_password: data.clear_password,
					mode: data.mode,
					renew_days_before: data.renew_days_before,
					remind_days_before: data.remind_days_before
				});
				toast(t.updated, 'ok');
			} else {
				await commands.accountCreate({
					library_base_url: url,
					library_name: data.library_name,
					login: data.login,
					password: data.password,
					mode: data.mode,
					renew_days_before: data.renew_days_before,
					remind_days_before: data.remind_days_before
				});
				toast(t.created, 'ok');
			}
			formOpen = false;
			await load();
			return true;
		} catch (e) {
			toast(describeError(e), 'danger');
			return false;
		}
	}

	async function openTest(a: Account_Serialize) {
		testing = a;
		quota = null;
		quotaFailed = false;
		testOpen = true;
		try {
			const q = await commands.quotaStatus(a.id);
			if (testing?.id === a.id) quota = q;
		} catch {
			if (testing?.id === a.id) quotaFailed = true;
		}
	}

	async function runTest() {
		if (!testing) return;
		testBusy = true;
		try {
			const ok = await commands.accountTestLogin(testing.id);
			toast(ok ? t.testOk : t.testFail, ok ? 'ok' : 'danger');
		} catch (e) {
			toast(describeError(e), 'danger');
		} finally {
			testBusy = false;
			testOpen = false;
		}
		await load();
	}

	function openRemove(a: Account_Serialize) {
		removing = a;
		removeAck = false;
		removeOpen = true;
	}

	async function runRemove() {
		if (!removing || !removeAck) return;
		removeBusy = true;
		try {
			await commands.accountDelete(removing.id);
			toast(t.removed, 'ok');
			removeOpen = false;
			await load();
			document.getElementById('contas-add')?.focus();
		} catch (e) {
			toast(describeError(e), 'danger');
		} finally {
			removeBusy = false;
		}
	}
</script>

<svelte:head><title>{t.title} · Renovador de Livros</title></svelte:head>

<header class="contas-head">
	<h1>{t.title}</h1>
	<p>{t.intro}</p>
	<div class="contas-head__bar">
		<span class="contas-count" aria-live="polite">{t.limit(accounts.length, MAX_ACCOUNTS)}</span>
		<Button
			id="contas-add"
			variant="primary"
			aria-disabled={full}
			aria-describedby={full ? 'contas-limit' : undefined}
			onclick={() => openForm()}
		>
			{t.add}
		</Button>
	</div>
	{#if full}<p id="contas-limit" class="contas-limit">{t.limitReached}</p>{/if}
</header>

{#if loading && accounts.length === 0}
	<p class="sr-only" role="status">{t.loading}</p>
	<Skeleton lines={4} />
{:else if loadError}
	<div role="alert" class="contas-error">
		<p>{loadError}</p>
		<Button onclick={load}>{t.retry}</Button>
	</div>
{:else if accounts.length === 0}
	<EmptyState title={t.emptyTitle} description={t.emptyDescription} />
{:else}
	<ul class="contas-list" aria-label={t.listLabel} bind:this={list}>
		{#each accounts as a (a.id)}
			<li>
				<Ficha title={a.login} subtitle={a.library_name}>
					<dl class="contas-meta">
						<dt>{t.statusLabel}</dt>
						<dd><StatusPill status={statusKind[a.status]} label={t.status[a.status]} /></dd>
						<dt>{t.mode}</dt>
						<dd>{t.modes[a.mode]}</dd>
						<dt>{t.daysLabel}</dt>
						<dd>{t.daysSummary(a.renew_days_before, a.remind_days_before)}</dd>
						<dt>{t.passwordLabel}</dt>
						<dd>{a.has_password ? t.hasPassword : t.noPassword}</dd>
						<dt>{t.lastSync}</dt>
						<dd class="mono">{formatDateTime(a.last_sync_at)}</dd>
					</dl>
					{#snippet actions()}
						<Button onclick={() => openForm(a)} aria-label={`${t.edit} ${a.login}`}>{t.edit}</Button>
						<Button onclick={() => openTest(a)} aria-label={`${t.testLogin} ${a.login}`}>{t.testLogin}</Button>
						<Button variant="danger" onclick={() => openRemove(a)} aria-label={`${t.remove} ${a.login}`}>
							{t.remove}
						</Button>
					{/snippet}
				</Ficha>
			</li>
		{/each}
	</ul>
{/if}

<Dialog bind:open={formOpen} title={editing ? t.editTitle(editing.login) : t.createTitle}>
	{#key formKey}
		{#if formOpen}
			<AccountForm account={editing} {presets} onsubmit={save} oncancel={() => (formOpen = false)} />
		{/if}
	{/key}
</Dialog>

<Dialog bind:open={testOpen} title={t.testTitle}>
	<p>{t.testQuota}</p>
	<p aria-live="polite">
		{#if quota}
			{t.testQuotaLeft(quota.manual_sessions_left)}
			{#if quota.next_allowed_at}{t.testQuotaNext(formatDateTime(quota.next_allowed_at))}{/if}
		{:else if quotaFailed}
			{t.testQuotaUnknown}
		{/if}
	</p>
	{#snippet actions()}
		<Button onclick={() => (testOpen = false)}>{t.cancel}</Button>
		<Button variant="primary" loading={testBusy} disabled={quota?.manual_sessions_left === 0} onclick={runTest}>
			{t.testConfirm}
		</Button>
	{/snippet}
</Dialog>

<Dialog bind:open={removeOpen} title={removing ? t.removeTitle(removing.login) : t.remove}>
	<p>{t.removeBody}</p>
	<Toggle label={t.removeAck} bind:checked={removeAck} />
	{#snippet actions()}
		<Button onclick={() => (removeOpen = false)}>{t.cancel}</Button>
		<Button variant="danger" loading={removeBusy} disabled={!removeAck} onclick={runRemove}>{t.removeConfirm}</Button>
	{/snippet}
</Dialog>

<style>
	.contas-head {
		display: grid;
		gap: var(--space-2);
		margin-block-end: var(--space-4);
	}
	.contas-head__bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.contas-count,
	.mono {
		font-family: var(--font-mono);
	}
	.contas-limit {
		color: var(--color-ink-muted);
	}
	.contas-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(20rem, 100%), 1fr));
		gap: var(--space-4);
	}
	.contas-meta {
		display: grid;
		grid-template-columns: max-content 1fr;
		align-items: center;
		gap: var(--space-1) var(--space-3);
		margin: 0;
	}
	.contas-meta dt {
		font-weight: 700;
	}
	.contas-meta dd {
		margin: 0;
	}
	.contas-error {
		display: grid;
		gap: var(--space-3);
		justify-items: start;
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
