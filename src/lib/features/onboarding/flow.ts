// Lógica do onboarding, isolada da UI para ser testável com o IPC mockado.
import { commands, type AccountMode, type LibraryPreset } from '#lib/ipc/index.ts';
import t from '#lib/i18n/pt-BR/onboarding.ts';

export const STEPS = ['privacy', 'library', 'account', 'test', 'notify'] as const;
export type StepId = (typeof STEPS)[number];

/** Endereço da UFTM, usado se a lista de presets não puder ser carregada. */
export const UFTM_FALLBACK: LibraryPreset = {
	name: 'UFTM',
	base_url: 'https://biblioteca.sophia.com.br/5362',
	supported: true
};

// Padrões da seção 5.4 (iguais aos da tela Contas): renova 1 dia antes, lembra 2 dias antes.
export const DEFAULT_RENEW_DAYS = 1;
export const DEFAULT_REMIND_DAYS = 2;

/** Escolhe o preset inicial: o primeiro suportado (UFTM). */
export function defaultPreset(presets: LibraryPreset[]): LibraryPreset {
	return presets.find((p) => p.supported) ?? presets[0] ?? UFTM_FALLBACK;
}

export async function loadPresets(): Promise<LibraryPreset[]> {
	try {
		const list = await commands.libraryPresets();
		return list.length ? list : [UFTM_FALLBACK];
	} catch {
		return [UFTM_FALLBACK];
	}
}

/** Converte um erro do IPC (`Error` tipado do backend) em texto pt-BR. */
export function errorMessage(e: unknown): string {
	const err = e as { kind?: string; message?: unknown } | null;
	switch (err?.kind) {
		case 'invalid_credentials':
			return t.errors.invalidCredentials;
		case 'human_verification_required':
			return t.errors.human;
		case 'network':
			return t.errors.network;
		case 'invalid_library':
			return t.library.invalid;
		case 'guard': {
			const reason = (err.message as { reason?: string } | undefined)?.reason;
			if (reason === 'account_limit') return t.errors.accountLimit;
			if (reason === 'quota_exhausted' || reason === 'session_budget_exhausted') return t.errors.quota;
			return t.errors.guard;
		}
		case 'invalid':
			return typeof err.message === 'string' && err.message ? err.message : t.errors.generic;
		default:
			return t.errors.generic;
	}
}

export type Validated = { ok: true; url: string } | { ok: false; error: string };

/** Valida um endereço customizado no backend. */
export async function validateLibrary(url: string): Promise<Validated> {
	const trimmed = url.trim();
	if (!trimmed) return { ok: false, error: t.library.required };
	try {
		const v = await commands.libraryValidate(trimmed);
		if (v.ok) return { ok: true, url: v.normalized_url ?? trimmed };
		return { ok: false, error: v.reason || t.library.invalid };
	} catch (e) {
		return { ok: false, error: errorMessage(e) };
	}
}

export interface AccountDraft {
	libraryUrl: string;
	libraryName: string;
	login: string;
	password: string;
	mode: AccountMode;
}

/** Cria a conta (ou atualiza a já criada). Retorna o id. */
export async function saveOnly(draft: AccountDraft, existingId: number | null): Promise<number> {
	if (existingId !== null) {
		await commands.accountUpdate(existingId, {
			library_base_url: draft.libraryUrl,
			library_name: draft.libraryName,
			login: draft.login.trim(),
			password: draft.password,
			clear_password: false,
			mode: draft.mode
		});
		return existingId;
	}
	const acc = await commands.accountCreate({
		library_base_url: draft.libraryUrl,
		library_name: draft.libraryName,
		login: draft.login.trim(),
		password: draft.password,
		mode: draft.mode,
		renew_days_before: DEFAULT_RENEW_DAYS,
		remind_days_before: DEFAULT_REMIND_DAYS
	});
	return acc.id;
}

/** Testa o login de uma conta já salva (conta na cota). */
export function testLogin(id: number): Promise<boolean> {
	return commands.accountTestLogin(id);
}

/** Pede permissão de notificação ao sistema. */
export async function requestNotifications(): Promise<boolean> {
	try {
		const { isPermissionGranted, requestPermission } = await import('@tauri-apps/plugin-notification');
		if (typeof window === 'undefined' || !('Notification' in window)) return false;
		if (await isPermissionGranted()) return true;
		return (await requestPermission()) === 'granted';
	} catch {
		return false;
	}
}

/** Grava as escolhas finais e marca o onboarding como concluído. */
export async function finish(notifications: boolean, privacyAcceptedAt: string): Promise<void> {
	await commands.settingsSet({
		notifications,
		onboarding_done: true,
		privacy_accepted_at: privacyAcceptedAt
	});
}
