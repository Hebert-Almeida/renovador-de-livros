<script lang="ts">
	import { onMount, tick } from 'svelte';
	import Button from '#lib/components/Button.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import Select from '#lib/components/Select.svelte';
	import Dialog from '#lib/components/Dialog.svelte';
	import TextField from '#lib/components/TextField.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import StatusPill, { type Status } from '#lib/components/StatusPill.svelte';
	import { toast } from '#lib/components/toast.svelte.ts';
	import { reveal } from '#lib/motion/index.ts';
	import { commands, type Schedule, type ScheduleStatus } from '#lib/ipc/bindings.ts';
	import t from '#lib/i18n/pt-BR/agenda.ts';
	import {
		actionsFor,
		formatRunAt,
		fromLocalInput,
		sortByRunAt,
		toLocalInput,
		validateRunAt,
		type Action
	} from './schedule.ts';

	type Filter = 'all' | ScheduleStatus;
	const filterOptions = (Object.keys(t.filters) as Filter[]).map((value) => ({ value, label: t.filters[value] }));
	const pillFor: Record<ScheduleStatus, Status> = {
		pending: 'info',
		paused: 'warn',
		cancelled: 'neutral',
		failed: 'danger',
		done: 'ok'
	};

	let filter = $state<Filter>('all');
	let items = $state<Schedule[]>([]);
	let loading = $state(true);
	let loadFailed = $state(false);
	let busyId = $state<number | null>(null);
	let list: HTMLElement | undefined = $state();

	let cancelTarget = $state<Schedule | null>(null);
	let cancelOpen = $state(false);
	let rescheduleTarget = $state<Schedule | null>(null);
	let rescheduleOpen = $state(false);
	let rescheduleValue = $state('');
	let rescheduleError = $state<string | undefined>();

	const visible = $derived(sortByRunAt(filter === 'all' ? items : items.filter((s) => s.status === filter)));

	const what = (s: Schedule) => t.kind[s.kind];
	const label = (a: Action, s: Schedule) => (a === 'cancel' && s.kind === 'renew' ? t.actions.cancelRenew : t.actions[a]);

	async function load() {
		loading = true;
		loadFailed = false;
		try {
			items = await commands.scheduleList({});
		} catch {
			loadFailed = true;
		} finally {
			loading = false;
		}
		await tick();
		if (list) reveal([...list.querySelectorAll('.agenda__item')]);
	}

	onMount(() => {
		void load();
	});

	function replace(updated: Schedule) {
		items = items.map((s) => (s.id === updated.id ? updated : s));
	}

	async function run(s: Schedule, action: Action, runAt?: string): Promise<boolean> {
		busyId = s.id;
		try {
			let updated: Schedule;
			if (action === 'pause') updated = await commands.schedulePause(s.id);
			else if (action === 'reactivate') updated = await commands.scheduleReactivate(s.id);
			else if (action === 'cancel') updated = await commands.scheduleCancel(s.id);
			else updated = await commands.scheduleReschedule(s.id, runAt!);
			replace(updated);
			toast(action === 'cancel' && s.kind === 'renew' ? t.done.cancelRenew : t.done[action], 'ok');
			return true;
		} catch {
			toast(t.actionError, 'danger');
			return false;
		} finally {
			busyId = null;
		}
	}

	function onAction(s: Schedule, action: Action) {
		if (action === 'cancel') {
			cancelTarget = s;
			cancelOpen = true;
		} else if (action === 'reschedule') {
			rescheduleTarget = s;
			rescheduleValue = toLocalInput(s.run_at);
			rescheduleError = undefined;
			rescheduleOpen = true;
		} else {
			void run(s, action);
		}
	}

	async function confirmCancel() {
		const s = cancelTarget;
		cancelOpen = false;
		if (s) await run(s, 'cancel');
	}

	async function submitReschedule(e: SubmitEvent) {
		e.preventDefault();
		const s = rescheduleTarget;
		if (!s || busyId !== null) return;
		const err = validateRunAt(rescheduleValue);
		if (err) {
			rescheduleError = t.rescheduleErrors[err];
			return;
		}
		rescheduleError = undefined;
		if (await run(s, 'reschedule', fromLocalInput(rescheduleValue)!)) rescheduleOpen = false;
	}
</script>

<p class="agenda__intro">{t.intro}</p>

<div class="agenda__filter">
	<Select label={t.filterLabel} options={filterOptions} bind:value={filter} />
</div>

{#if loading}
	<div role="status" aria-label={t.loading}><Skeleton lines={3} /></div>
{:else if loadFailed}
	<div role="alert" class="agenda__error">
		<p>{t.loadError}</p>
		<Button onclick={load}>{t.retry}</Button>
	</div>
{:else if visible.length === 0}
	<EmptyState title={t.emptyTitle} description={t.emptyDescription} />
{:else}
	<ul class="agenda__list" aria-label={t.listLabel} bind:this={list}>
		{#each visible as s (s.id)}
			{@const when = formatRunAt(s.run_at)}
			<li class="agenda__item">
				<Ficha
					title={`${what(s)} · ${when}`}
					subtitle={[t.account(s.account_id), s.loan_id != null ? t.loan(s.loan_id) : null, t.origin[s.origin]]
						.filter(Boolean)
						.join(' · ')}
				>
					<p class="agenda__meta">
						<StatusPill status={pillFor[s.status]} label={t.status[s.status]} />
						<span>{t.runAt} <time class="agenda__time" datetime={s.run_at}>{when}</time></span>
					</p>
					{#snippet actions()}
						{#each actionsFor(s.status) as a (a)}
							<Button
								variant={a === 'cancel' ? 'danger' : a === 'reactivate' ? 'primary' : 'secondary'}
								aria-label={t.actionLabel(label(a, s), what(s), when)}
								loading={busyId === s.id}
								disabled={busyId !== null && busyId !== s.id}
								onclick={() => onAction(s, a)}>{label(a, s)}</Button
							>
						{/each}
					{/snippet}
				</Ficha>
			</li>
		{/each}
	</ul>
{/if}

<Dialog bind:open={cancelOpen} title={cancelTarget?.kind === 'renew' ? t.cancelRenewTitle : t.cancelTitle}>
	{#if cancelTarget}<p>{t.cancelBody(what(cancelTarget), formatRunAt(cancelTarget.run_at))}</p>{/if}
	{#snippet actions()}
		<Button onclick={() => (cancelOpen = false)}>{t.keep}</Button>
		<Button variant="danger" onclick={confirmCancel}>{t.confirmCancel}</Button>
	{/snippet}
</Dialog>

<Dialog bind:open={rescheduleOpen} title={t.rescheduleTitle}>
	<form id="agenda-reschedule" novalidate onsubmit={submitReschedule}>
		<TextField
			label={t.rescheduleField}
			hint={t.rescheduleHint}
			type="datetime-local"
			bind:value={rescheduleValue}
			error={rescheduleError}
		/>
	</form>
	{#snippet actions()}
		<Button onclick={() => (rescheduleOpen = false)}>{t.close}</Button>
		<Button variant="primary" type="submit" form="agenda-reschedule" loading={busyId !== null}>{t.save}</Button>
	{/snippet}
</Dialog>

<style>
	.agenda__intro {
		max-width: 60ch;
	}
	.agenda__filter {
		max-width: 18rem;
		margin-block: var(--space-4);
	}
	.agenda__list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 22rem), 1fr));
		gap: var(--space-4);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.agenda__meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
		margin: 0;
	}
	.agenda__time {
		font-family: var(--font-mono);
	}
	.agenda__error {
		display: grid;
		gap: var(--space-3);
		justify-items: start;
	}
</style>
