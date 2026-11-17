<script lang="ts">
	import { onMount } from 'svelte';
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import t from '#lib/i18n/pt-BR/ajustes.ts';
	import { commands, type Settings_Serialize, type SettingsPatch_Deserialize } from '#lib/ipc/bindings.ts';
	import Button from '#lib/components/Button.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import Toggle from '#lib/components/Toggle.svelte';
	import Select from '#lib/components/Select.svelte';
	import TextField from '#lib/components/TextField.svelte';
	import PasswordField from '#lib/components/PasswordField.svelte';
	import Dialog from '#lib/components/Dialog.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import { toast } from '#lib/components/toast.svelte.ts';
	import { reveal } from '#lib/motion/index.ts';
	import {
		THEMES,
		MOTIONS,
		PERIODS,
		isPeriod,
		applyTheme,
		applyMotion,
		isTheme,
		isMotion,
		passphraseOk,
		errorText
	} from '#lib/features/ajustes/settings.ts';

	type Flag = 'notifications' | 'autostart' | 'app_lock';

	let settings: Settings_Serialize | null = $state(null);
	let loadFailed = $state(false);
	let theme = $state('system');
	let motion = $state('system');
	let period = $state('auto');
	let flags = $state({ notifications: false, autostart: false, app_lock: false });

	let path = $state('');
	let passphrase = $state('');
	let pathError = $state('');
	let passError = $state('');
	let busy: 'create' | 'restore' | 'wipe' | null = $state(null);
	let restoreOpen = $state(false);
	let wipeOpen = $state(false);
	let wipeText = $state('');
	let cards: HTMLElement | undefined = $state();

	const themeOptions = THEMES.map((v) => ({ value: v, label: t.appearance.themes[v] }));
	const motionOptions = MOTIONS.map((v) => ({ value: v, label: t.appearance.motions[v] }));
	const periodOptions = PERIODS.map((v) => ({ value: v, label: t.schedule.periods[v] }));
	const wipeReady = $derived(wipeText.trim() === t.data.wipeWord);

	function adopt(s: Settings_Serialize) {
		settings = s;
		theme = s.theme;
		motion = s.motion;
		period = s.renew_period;
		flags = { notifications: s.notifications, autostart: s.autostart, app_lock: s.app_lock };
		applyTheme(s.theme);
		applyMotion(s.motion);
	}

	async function load() {
		loadFailed = false;
		try {
			adopt(await commands.settingsGet());
		} catch {
			loadFailed = true;
		}
	}

	onMount(() => {
		void load();
	});

	let revealed = false;
	$effect(() => {
		if (cards && !revealed) {
			revealed = true;
			reveal([...cards.children]);
		}
	});

	async function update(patch: SettingsPatch_Deserialize): Promise<boolean> {
		try {
			settings = await commands.settingsSet(patch);
			toast(t.saved, 'ok', 2500);
			return true;
		} catch {
			toast(t.saveError, 'danger');
			return false;
		}
	}

	async function onTheme(value: string) {
		if (!settings || !isTheme(value) || value === settings.theme) return;
		const previous = settings.theme;
		theme = value;
		applyTheme(value);
		if (!(await update({ theme: value }))) {
			theme = previous;
			applyTheme(previous);
		}
	}

	async function onMotion(value: string) {
		if (!settings || !isMotion(value) || value === settings.motion) return;
		const previous = settings.motion;
		motion = value;
		applyMotion(value);
		if (!(await update({ motion: value }))) {
			motion = previous;
			applyMotion(previous);
		}
	}

	async function onPeriod(value: string) {
		if (!settings || !isPeriod(value) || value === settings.renew_period) return;
		const previous = settings.renew_period;
		period = value;
		if (!(await update({ renew_period: value }))) period = previous;
	}

	async function onFlag(key: Flag, value: boolean) {
		if (!(await update({ [key]: value }))) flags[key] = !value;
	}

	function validateBackup(): boolean {
		pathError = path.trim() ? '' : t.data.pathMissing;
		passError = passphraseOk(passphrase) ? '' : t.data.passphraseShort;
		return !pathError && !passError;
	}

	async function pickFile() {
		try {
			const p = await saveDialog({ defaultPath: path || 'renovador.bak' });
			if (p) path = p;
		} catch {
			// Sem diálogo nativo (ex.: navegador): o campo de texto continua disponível.
		}
	}

	async function createBackup() {
		if (!validateBackup()) return;
		busy = 'create';
		try {
			await commands.backupCreate(path.trim(), passphrase);
			toast(t.data.created, 'ok');
			passphrase = '';
		} catch (e) {
			toast(`${t.data.failed}: ${errorText(e)}`, 'danger');
		} finally {
			busy = null;
		}
	}

	function askRestore() {
		if (validateBackup()) restoreOpen = true;
	}

	async function restoreBackup() {
		busy = 'restore';
		try {
			await commands.backupRestore(path.trim(), passphrase);
			passphrase = '';
			toast(t.data.restored, 'ok');
			await load();
		} catch (e) {
			toast(`${t.data.failed}: ${errorText(e)}`, 'danger');
		} finally {
			restoreOpen = false;
			busy = null;
		}
	}

	function askWipe() {
		wipeText = '';
		wipeOpen = true;
	}

	async function wipe() {
		if (!wipeReady) return;
		busy = 'wipe';
		try {
			await commands.dataWipe();
			toast(t.data.wiped, 'ok');
			await load();
		} catch (e) {
			toast(`${t.data.failed}: ${errorText(e)}`, 'danger');
		} finally {
			wipeOpen = false;
			busy = null;
		}
	}
</script>

<svelte:head><title>{t.title} · Renovador de Livros</title></svelte:head>
<h1>{t.title}</h1>
<p class="intro">{t.intro}</p>

{#if loadFailed}
	<div role="alert" class="load-error">
		<p>{t.loadError}</p>
		<Button onclick={load}>{t.retry}</Button>
	</div>
{:else if !settings}
	<p class="visually-hidden" role="status">{t.loading}</p>
	<Skeleton lines={6} />
{:else}
	<div class="cards" bind:this={cards}>
		<Ficha title={t.schedule.heading}>
			<h3 class="sub">{t.schedule.rules}</h3>
			<ul class="rules">
				<li>{t.schedule.renewBefore}</li>
				<li>{t.schedule.retry}</li>
				<li>{t.schedule.reminders}</li>
				<li>{t.schedule.window}</li>
				<li>{t.schedule.sync}</li>
			</ul>
			<Select
				label={t.schedule.period}
				hint={t.schedule.periodHint}
				options={periodOptions}
				value={period}
				onchange={(e) => onPeriod(e.currentTarget.value)}
			/>
			<Toggle
				label={t.schedule.notifications}
				description={t.schedule.notificationsHint}
				bind:checked={flags.notifications}
				onchange={(v) => onFlag('notifications', v)}
			/>
		</Ficha>

		<Ficha title={t.appearance.heading}>
			<div class="grid">
				<Select
					label={t.appearance.theme}
					options={themeOptions}
					value={theme}
					onchange={(e) => onTheme(e.currentTarget.value)}
				/>
				<Select
					label={t.appearance.motion}
					options={motionOptions}
					value={motion}
					onchange={(e) => onMotion(e.currentTarget.value)}
				/>
			</div>
		</Ficha>

		<Ficha title={t.system.heading}>
			<div class="stack">
				<Toggle
					label={t.system.autostart}
					description={t.system.autostartHint}
					bind:checked={flags.autostart}
					onchange={(v) => onFlag('autostart', v)}
				/>
				<Toggle
					label={t.system.appLock}
					description={t.system.appLockHint}
					bind:checked={flags.app_lock}
					onchange={(v) => onFlag('app_lock', v)}
				/>
			</div>
		</Ficha>

		<Ficha title={t.data.heading}>
			<div class="grid">
				<TextField
					label={t.data.path}
					hint={t.data.pathHint}
					error={pathError || undefined}
					bind:value={path}
					autocomplete="off"
				/>
				<PasswordField
					label={t.data.passphrase}
					hint={t.data.passphraseHint}
					error={passError || undefined}
					bind:value={passphrase}
					autocomplete="new-password"
				/>
			</div>
			{#snippet actions()}
				<Button onclick={pickFile}>{t.data.pick}</Button>
				<Button variant="primary" loading={busy === 'create'} disabled={busy !== null} onclick={createBackup}>
					{t.data.create}
				</Button>
				<Button disabled={busy !== null} onclick={askRestore}>{t.data.restore}</Button>
				<Button variant="danger" disabled={busy !== null} onclick={askWipe}>{t.data.wipe}</Button>
			{/snippet}
		</Ficha>
	</div>

	<Dialog bind:open={restoreOpen} title={t.data.restoreTitle}>
		<p>{t.data.restoreBody}</p>
		{#snippet actions()}
			<Button onclick={() => (restoreOpen = false)}>{t.data.cancel}</Button>
			<Button variant="primary" loading={busy === 'restore'} onclick={restoreBackup}>{t.data.restoreConfirm}</Button>
		{/snippet}
	</Dialog>

	<Dialog bind:open={wipeOpen} title={t.data.wipeTitle}>
		<p>{t.data.wipeBody}</p>
		<TextField label={t.data.wipeLabel} bind:value={wipeText} autocomplete="off" spellcheck={false} />
		{#snippet actions()}
			<Button onclick={() => (wipeOpen = false)}>{t.data.cancel}</Button>
			<Button variant="danger" loading={busy === 'wipe'} disabled={!wipeReady} onclick={wipe}>
				{t.data.wipeConfirm}
			</Button>
		{/snippet}
	</Dialog>
{/if}

<style>
	.intro {
		margin-block-end: var(--space-4);
	}
	.cards {
		display: grid;
		gap: var(--space-5);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
		gap: var(--space-4);
	}
	.stack {
		display: grid;
		gap: var(--space-4);
	}
	.rules {
		margin: 0 0 var(--space-4);
		padding-inline-start: 1.25rem;
	}
	.sub {
		margin: 0 0 var(--space-2);
	}
	.load-error {
		display: grid;
		gap: var(--space-3);
		justify-items: start;
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
