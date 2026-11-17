// Lógica pura da tela de ajustes: aplicação de tema/movimento e validações.
import type { MotionPref, Theme } from '#lib/ipc/bindings.ts';

export const THEMES: readonly Theme[] = ['system', 'light', 'dark', 'contrast'];
export const MOTIONS: readonly MotionPref[] = ['system', 'reduced', 'full'];
export const MIN_PASSPHRASE = 8;

/** Aplica o tema em `<html data-theme>`; "system" remove o atributo (tokens.css segue o SO). */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
	if (theme === 'system') delete root.dataset.theme;
	else root.dataset.theme = theme;
}

/** Aplica a preferência de movimento em `<html data-motion>`; "system" remove o atributo. */
export function applyMotion(motion: MotionPref, root: HTMLElement = document.documentElement): void {
	if (motion === 'system') delete root.dataset.motion;
	else root.dataset.motion = motion;
}

export function isTheme(v: string): v is Theme {
	return (THEMES as readonly string[]).includes(v);
}

export function isMotion(v: string): v is MotionPref {
	return (MOTIONS as readonly string[]).includes(v);
}

export function passphraseOk(p: string): boolean {
	return p.length >= MIN_PASSPHRASE;
}

/** Extrai uma mensagem legível do erro do IPC (`{ kind, message? }`) ou de um Error. */
export function errorText(e: unknown): string {
	if (typeof e === 'string') return e;
	if (e && typeof e === 'object') {
		const m = (e as { message?: unknown }).message;
		if (typeof m === 'string') return m;
		const k = (e as { kind?: unknown }).kind;
		if (typeof k === 'string') return k;
	}
	return String(e);
}
