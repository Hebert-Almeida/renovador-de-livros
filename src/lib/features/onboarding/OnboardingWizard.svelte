<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import Button from '#lib/components/Button.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import TextField from '#lib/components/TextField.svelte';
	import PasswordField from '#lib/components/PasswordField.svelte';
	import Toggle from '#lib/components/Toggle.svelte';
	import ProgressSteps, { type Step } from '#lib/components/ProgressSteps.svelte';
	import StatusPill from '#lib/components/StatusPill.svelte';
	import HeroBook from '#lib/three/HeroBook.svelte';
	import { reveal } from '#lib/motion/index.ts';
	import type { AccountMode, LibraryPreset } from '#lib/ipc/index.ts';
	import t from '#lib/i18n/pt-BR/onboarding.ts';
	import {
		STEPS,
		type StepId,
		UFTM_FALLBACK,
		defaultPreset,
		loadPresets,
		validateLibrary,
		saveOnly,
		testLogin,
		requestNotifications,
		finish,
		errorMessage
	} from './flow.ts';

	let { onDone }: { onDone?: () => void } = $props();

	let index = $state(0);
	const step: StepId = $derived(STEPS[index]);
	let busy = $state(false);
	let error = $state('');
	let heading: HTMLElement | undefined = $state();
	let panel: HTMLElement | undefined = $state();

	// Privacidade
	let consent = $state(false);
	let acceptedAt = $state('');

	// Biblioteca
	let presets: LibraryPreset[] = $state([]);
	let presetUrl = $state(UFTM_FALLBACK.base_url);
	let advanced = $state(false);
	let customUrl = $state('');
	let resolvedUrl = $state('');

	// Conta
	let login = $state('');
	let password = $state('');
	let mode: AccountMode = $state('auto');
	let accountId: number | null = $state(null);
	let loginError = $state('');
	let passwordError = $state('');

	// Teste e notificações
	let testResult: 'idle' | 'ok' | 'fail' = $state('idle');
	let notifications: boolean | null = $state(null);

	const progress: Step[] = $derived(
		STEPS.map((id, i) => ({
			id,
			label: t.steps[id],
			state: i < index ? 'done' : i === index ? 'active' : 'pending'
		}))
	);

	const libraryName = $derived(
		advanced ? t.library.customName : (presets.find((p) => p.base_url === presetUrl)?.name ?? UFTM_FALLBACK.name)
	);
	const draft = $derived({ libraryUrl: resolvedUrl, libraryName, login, password, mode });

	onMount(async () => {
		presets = await loadPresets();
		presetUrl = defaultPreset(presets).base_url;
	});

	async function go(to: number) {
		error = '';
		index = to;
		await tick();
		heading?.focus();
		if (panel) reveal(panel);
	}

	async function next() {
		error = '';
		busy = true;
		try {
			if (step === 'privacy') {
				if (!consent) {
					error = t.privacy.required;
					return;
				}
				acceptedAt = new Date().toISOString();
			} else if (step === 'library') {
				if (advanced) {
					const v = await validateLibrary(customUrl);
					if (!v.ok) {
						error = v.error;
						return;
					}
					resolvedUrl = v.url;
				} else {
					resolvedUrl = presetUrl;
				}
			} else if (step === 'account') {
				loginError = login.trim() ? '' : t.account.requiredLogin;
				passwordError = password ? '' : t.account.requiredPassword;
				if (loginError || passwordError) return;
				accountId = await saveOnly(draft, accountId);
				testResult = 'idle';
			} else if (step === 'notify') {
				await finish(notifications === true, acceptedAt);
				if (onDone) onDone();
				else await goto('/');
				return;
			}
			await go(index + 1);
		} catch (e) {
			error = errorMessage(e);
		} finally {
			busy = false;
		}
	}

	async function runTest() {
		error = '';
		busy = true;
		try {
			if (accountId === null) accountId = await saveOnly(draft, null);
			testResult = (await testLogin(accountId)) ? 'ok' : 'fail';
		} catch (e) {
			testResult = 'fail';
			error = errorMessage(e);
		} finally {
			busy = false;
		}
	}

	async function askNotifications() {
		busy = true;
		try {
			notifications = await requestNotifications();
		} finally {
			busy = false;
		}
	}
</script>

<div class="onboarding">
	<header class="onboarding__head">
		<div class="onboarding__intro">
			<h1>{t.title}</h1>
			<p>{t.intro}</p>
			<ProgressSteps label={t.progressLabel} steps={progress} />
		</div>
		<div class="onboarding__hero"><HeroBook /></div>
	</header>

	<form
		class="onboarding__panel"
		novalidate
		bind:this={panel}
		onsubmit={(e) => {
			e.preventDefault();
			if (!busy) next();
		}}
	>
		<Ficha title={t.stepOf(index + 1, STEPS.length)} subtitle={t.steps[step]}>
			<h3 class="step-heading" tabindex="-1" bind:this={heading}>
				{#if step === 'privacy'}{t.privacy.heading}
				{:else if step === 'library'}{t.library.heading}
				{:else if step === 'account'}{t.account.heading}
				{:else if step === 'test'}{t.test.heading}
				{:else}{t.notify.heading}{/if}
			</h3>

			{#if step === 'privacy'}
				<ul class="notice">
					{#each t.privacy.items as item (item)}<li>{item}</li>{/each}
				</ul>
				<label class="check">
					<input type="checkbox" bind:checked={consent} aria-describedby={error ? 'ob-error' : undefined} />
					<span>{t.privacy.consent}</span>
				</label>
			{:else if step === 'library'}
				<fieldset class="choices" disabled={advanced}>
					<legend>{t.library.legend}</legend>
					{#if presets.length === 0}
						<p>{t.library.loading}</p>
					{/if}
					{#each presets as p (p.base_url)}
						<label class="choice">
							<input type="radio" name="library" value={p.base_url} bind:group={presetUrl} />
							<span class="choice__text">
								<strong>{p.name}</strong>
								<span class="mono">{p.base_url}</span>
							</span>
							<StatusPill
								status={p.supported ? 'ok' : 'warn'}
								label={p.supported ? t.library.supported : t.library.experimental}
							/>
						</label>
					{/each}
				</fieldset>
				<Toggle label={t.library.advanced} description={t.library.advancedHint} bind:checked={advanced} />
				{#if advanced}
					<p class="warning" role="note"><strong aria-hidden="true">!</strong> {t.library.warning}</p>
					<TextField
						label={t.library.urlLabel}
						hint={t.library.urlHint}
						type="url"
						inputmode="url"
						autocomplete="url"
						bind:value={customUrl}
					/>
				{/if}
			{:else if step === 'account'}
				<TextField
					label={t.account.login}
					autocomplete="username"
					bind:value={login}
					error={loginError || undefined}
				/>
				<PasswordField
					label={t.account.password}
					hint={t.account.passwordHint}
					bind:value={password}
					error={passwordError || undefined}
				/>
				<fieldset class="choices">
					<legend>{t.account.modeLegend}</legend>
					<label class="choice">
						<input type="radio" name="mode" value="auto" bind:group={mode} />
						<span class="choice__text"><strong>{t.account.auto}</strong><span>{t.account.autoHint}</span></span>
					</label>
					<label class="choice">
						<input type="radio" name="mode" value="manual" bind:group={mode} />
						<span class="choice__text"><strong>{t.account.manual}</strong><span>{t.account.manualHint}</span></span>
					</label>
				</fieldset>
			{:else if step === 'test'}
				<p>{t.test.explain}</p>
				<div class="row">
					<Button onclick={runTest} loading={busy}>{t.test.button}</Button>
				</div>
				<div aria-live="polite" class="live">
					{#if testResult === 'ok'}
						<StatusPill status="ok" label={t.test.ok} />
					{:else if testResult === 'fail' && !error}
						<StatusPill status="danger" label={t.test.fail} />
					{/if}
				</div>
				{#if testResult !== 'ok'}<p class="muted">{t.test.skipNote}</p>{/if}
			{:else}
				<p>{t.notify.explain}</p>
				<div class="row">
					<Button onclick={askNotifications} loading={busy} disabled={notifications === true}>{t.notify.allow}</Button>
				</div>
				<div aria-live="polite" class="live">
					{#if notifications === true}
						<StatusPill status="ok" label={t.notify.granted} />
					{:else if notifications === false}
						<StatusPill status="warn" label={t.notify.denied} />
					{/if}
				</div>
			{/if}

			<div class="live" role="alert" id="ob-error">
				{#if error}<p class="error"><strong aria-hidden="true">✕</strong> {error}</p>{/if}
			</div>

			{#snippet actions()}
				{#if index > 0}
					<Button variant="ghost" disabled={busy} onclick={() => go(index - 1)}>{t.back}</Button>
				{/if}
				<Button variant="primary" type="submit" loading={busy && step !== 'test' && step !== 'notify'} disabled={busy}>
					{step === 'notify' ? t.finish : t.next}
				</Button>
			{/snippet}
		</Ficha>
	</form>
</div>

<style>
	.onboarding {
		display: grid;
		gap: var(--space-5);
		max-width: 60rem;
	}
	.onboarding__head {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(8rem, 12rem);
		gap: var(--space-5);
		align-items: center;
	}
	.onboarding__intro {
		display: grid;
		gap: var(--space-3);
	}
	.step-heading {
		font-size: var(--text-lg);
		margin: 0;
	}
	.step-heading:focus-visible {
		outline: var(--focus-ring);
		outline-offset: var(--focus-offset);
	}
	.notice {
		display: grid;
		gap: var(--space-2);
		padding-left: 1.25rem;
	}
	.check,
	.choice {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		min-height: var(--target-min);
		cursor: pointer;
	}
	.choice {
		border: var(--border-width-thin) solid var(--color-rule);
		background: var(--color-paper-raised);
		padding: var(--space-2) var(--space-3);
	}
	.choice:has(input:checked) {
		box-shadow: var(--shadow-hard);
	}
	.check input,
	.choice input {
		width: 1.5rem;
		height: 1.5rem;
		flex: none;
		accent-color: var(--color-ink);
	}
	.check input:focus-visible,
	.choice input:focus-visible {
		outline: var(--focus-ring);
		outline-offset: var(--focus-offset);
	}
	.choice__text {
		display: grid;
		flex: 1;
	}
	.choices {
		display: grid;
		gap: var(--space-2);
		border: 0;
		padding: 0;
		margin: 0;
	}
	.choices:disabled {
		opacity: 0.6;
	}
	legend {
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		margin-bottom: var(--space-2);
	}
	.mono {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		overflow-wrap: anywhere;
	}
	.warning {
		border: var(--border);
		background: var(--color-warn-bg);
		color: var(--color-ink);
		padding: var(--space-3);
		margin: 0;
	}
	.error {
		color: var(--color-danger);
		font-weight: var(--weight-bold);
		margin: 0;
	}
	.muted {
		color: var(--color-ink-muted);
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	@media (max-width: 40rem) {
		.onboarding__head {
			grid-template-columns: 1fr;
		}
		.onboarding__hero {
			max-width: 10rem;
		}
	}
</style>
