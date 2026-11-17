// Converte erros do IPC em mensagens pt-BR. Nunca inclui dados sensíveis.
import t from '#lib/i18n/pt-BR/contas.ts';
import type { Error as IpcError } from '#lib/ipc/bindings.ts';

export const MAX_ACCOUNTS = 3;

function isIpcError(e: unknown): e is IpcError {
	return typeof e === 'object' && e !== null && 'kind' in e;
}

export function describeError(e: unknown): string {
	if (!isIpcError(e)) return t.errors.generic;
	switch (e.kind) {
		case 'invalid_credentials':
		case 'human_verification_required':
		case 'network':
		case 'invalid_library':
		case 'not_found':
			return t.errors[e.kind];
		case 'invalid':
			return e.message || t.errors.generic;
		case 'guard': {
			const r = e.message.reason;
			if (r === 'account_limit') return t.errors.account_limit;
			if (r === 'quota_exhausted' || r === 'session_budget_exhausted') return t.errors.quota;
			return t.errors.guard;
		}
		default:
			return t.errors.generic;
	}
}

export function formatDateTime(iso: string | null | undefined): string {
	if (!iso) return t.never;
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toLocaleString('pt-BR', { dateStyle: 'short', timeStyle: 'short' });
}
