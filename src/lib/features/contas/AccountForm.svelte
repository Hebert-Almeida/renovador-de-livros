<script lang="ts" module>
	import type { AccountMode } from '#lib/ipc/bindings.ts';

	export interface AccountFormData {
		library_base_url: string;
		library_name: string;
		login: string;
		/** `null` = manter (edição) ou não salvar (criação). */
		password: string | null;
		clear_password: boolean;
		mode: AccountMode;
	}
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import Button from '#lib/components/Button.svelte';
	import TextField from '#lib/components/TextField.svelte';
	import PasswordField from '#lib/components/PasswordField.svelte';
	import Select from '#lib/components/Select.svelte';
	import Toggle from '#lib/components/Toggle.svelte';
	import t from '#lib/i18n/pt-BR/contas.ts';
	import type { Account_Serialize, LibraryPreset } from '#lib/ipc/bindings.ts';

	const CUSTOM = '__custom__';

	let {
		account,
		presets,
		onsubmit,
		oncancel
	}: {
		account?: Account_Serialize;
		presets: LibraryPreset[];
		onsubmit: (data: AccountFormData) => Promise<boolean>;
		oncancel: () => void;
	} = $props();

	// Valores iniciais copiados uma vez: o formulário é recriado a cada abertura.
	const initial = untrack(() => account);
	const initialPresets = untrack(() => presets);
	const known = initial ? initialPresets.find((p) => p.base_url === initial.library_base_url) : initialPresets[0];
	let library = $state(initial && !known ? CUSTOM : (known?.base_url ?? CUSTOM));
	let customUrl = $state(initial && !known ? initial.library_base_url : '');
	let login = $state(initial?.login ?? '');
	let password = $state('');
	let clearPassword = $state(false);
	let mode = $state<string>(initial?.mode ?? 'auto');
	let submitted = $state(false);
	let busy = $state(false);

	const options = $derived([
		...presets.map((p) => ({ value: p.base_url, label: p.name + (p.supported ? '' : t.experimental) })),
		{ value: CUSTOM, label: t.customLibrary }
	]);
	const loginError = $derived(submitted && !login.trim() ? t.required : undefined);
	const urlError = $derived(submitted && library === CUSTOM && !customUrl.trim() ? t.required : undefined);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		if (loginError || urlError) return;
		const preset = presets.find((p) => p.base_url === library);
		const url = preset ? preset.base_url : customUrl.trim();
		busy = true;
		try {
			const ok = await onsubmit({
				library_base_url: url,
				library_name: preset?.name ?? (initial && !known ? initial.library_name : url),
				login: login.trim(),
				password: password && !clearPassword ? password : null,
				clear_password: clearPassword,
				mode: mode === 'manual' ? 'manual' : 'auto'
			});
			if (ok) password = '';
		} finally {
			busy = false;
		}
	}
</script>

<form class="account-form" onsubmit={submit} novalidate>
	<Select label={t.libraryOption} {options} bind:value={library} />
	{#if library === CUSTOM}
		<TextField label={t.customUrl} type="url" hint={t.customUrlHint} error={urlError} bind:value={customUrl} inputmode="url" />
	{/if}
	<TextField label={t.login} hint={t.loginHint} error={loginError} bind:value={login} autocomplete="username" required />
	<PasswordField
		label={initial ? t.passwordNew : t.password}
		hint={initial ? t.passwordKeepHint : t.passwordOptionalHint}
		autocomplete={initial ? 'new-password' : 'current-password'}
		disabled={clearPassword}
		bind:value={password}
	/>
	{#if initial?.has_password}
		<Toggle label={t.clearPassword} description={t.clearPasswordDescription} bind:checked={clearPassword} />
	{/if}
	<Select
		label={t.mode}
		options={[
			{ value: 'auto', label: t.modes.auto },
			{ value: 'manual', label: t.modes.manual }
		]}
		bind:value={mode}
	/>
	<div class="account-form__actions">
		<Button onclick={oncancel}>{t.cancel}</Button>
		<Button type="submit" variant="primary" loading={busy}>{t.save}</Button>
	</div>
</form>

<style>
	.account-form {
		display: grid;
		gap: var(--space-3);
		min-width: min(28rem, 80vw);
	}
	.account-form__actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
</style>
