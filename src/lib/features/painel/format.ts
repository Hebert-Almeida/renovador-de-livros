// Funções puras de formatação e regras de exibição do Painel.
import type { GuardError, QuotaStatus } from '#lib/ipc/bindings.ts';
import t from '#lib/i18n/pt-BR/painel.ts';

const MS_DAY = 86_400_000;

/** Converte `AAAA-MM-DD` (ou ISO completo) numa data local à meia-noite. */
const DAY_RE = /^(\d{4})-(\d{2})-(\d{2})/;

function localDay(value: string): Date {
	const m = DAY_RE.exec(value);
	if (!m) return new Date(NaN);
	return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
}

/** Dias corridos de `now` até `due` (negativo se já passou). */
export function daysUntil(due: string, now: Date = new Date()): number {
	const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
	return Math.round((localDay(due).getTime() - today.getTime()) / MS_DAY);
}

export function dueLabel(due: string, now: Date = new Date()): string {
	const d = daysUntil(due, now);
	if (d < 0) return t.overdue(-d);
	if (d === 0) return t.dueToday;
	if (d === 1) return t.dueTomorrow;
	return t.dueIn(d);
}

export function dueTone(due: string, now: Date = new Date()): 'ok' | 'warn' | 'danger' {
	const d = daysUntil(due, now);
	if (d < 0) return 'danger';
	if (d <= 2) return 'warn';
	return 'ok';
}

/** `AAAA-MM-DD` → `DD/MM/AAAA`. */
export function formatDate(value: string): string {
	const m = DAY_RE.exec(value);
	return m ? `${m[3]}/${m[2]}/${m[1]}` : value;
}

const dateTime = new Intl.DateTimeFormat('pt-BR', { dateStyle: 'short', timeStyle: 'short' });
const timeOnly = new Intl.DateTimeFormat('pt-BR', { hour: '2-digit', minute: '2-digit' });

export function formatDateTime(iso: string): string {
	const d = new Date(iso);
	return Number.isNaN(d.getTime()) ? iso : dateTime.format(d);
}

/** "às 14:30" se for hoje; senão "em 16/11/2026 14:30". */
export function formatWhen(iso: string, now: Date = new Date()): string {
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toDateString() === now.toDateString() ? `às ${timeOnly.format(d)}` : `em ${dateTime.format(d)}`;
}

export interface QuotaView {
	canSync: boolean;
	/** Texto da cota/espera, sempre presente. */
	text: string;
}

/** Decide se "Sincronizar agora" está liberado e o que dizer sobre cota e espera. */
export function quotaView(q: QuotaStatus | undefined, now: Date = new Date()): QuotaView {
	if (!q) return { canSync: true, text: '' };
	if (q.breaker_open_until && new Date(q.breaker_open_until) > now)
		return { canSync: false, text: t.breakerOpen(formatWhen(q.breaker_open_until, now)) };
	if (q.manual_sessions_left <= 0) {
		const extra = q.next_allowed_at && new Date(q.next_allowed_at) > now ? ' ' + t.waitUntil(formatWhen(q.next_allowed_at, now)) : '';
		return { canSync: false, text: t.quotaExhausted + extra };
	}
	if (q.next_allowed_at && new Date(q.next_allowed_at) > now)
		return { canSync: false, text: `${t.quotaLeft(q.manual_sessions_left)} ${t.waitUntil(formatWhen(q.next_allowed_at, now))}` };
	return { canSync: true, text: t.quotaLeft(q.manual_sessions_left) };
}

/** Mensagem legível para um erro do IPC (`Error` do bindings). */
export function errorMessage(e: unknown, now: Date = new Date()): string {
	if (e && typeof e === 'object' && 'kind' in e) {
		const err = e as { kind: string; message?: unknown };
		if (err.kind === 'guard' && err.message && typeof err.message === 'object') {
			const g = err.message as GuardError;
			switch (g.reason) {
				case 'quota_exhausted':
					return t.quotaExhausted;
				case 'too_soon':
					return t.waitUntil(formatWhen(g.retry_at, now));
				case 'breaker_open':
				case 'retry_after':
					return t.breakerOpen(formatWhen(g.until, now));
				case 'outside_window':
					return `Fora da janela permitida. ${t.waitUntil(formatWhen(g.next_window, now))}`;
				case 'remote_disabled':
					return t.remoteOff;
				default:
					return t.syncFailed;
			}
		}
		if (err.kind === 'invalid_credentials') return 'Login ou senha da biblioteca incorretos.';
		if (err.kind === 'human_verification_required') return 'A biblioteca pediu uma verificação humana. Entre pelo site e tente depois.';
		if (err.kind === 'network') return 'Sem conexão com a biblioteca.';
		if (typeof err.message === 'string' && err.message) return `${t.syncFailed} ${err.message}`;
	}
	return t.syncFailed;
}

/** Menor data ISO (ou `null`). */
export function earliest(values: (string | null | undefined)[]): string | null {
	const v = values.filter((x): x is string => !!x).sort();
	return v[0] ?? null;
}

/** Maior data ISO (ou `null`). */
export function latest(values: (string | null | undefined)[]): string | null {
	const v = values.filter((x): x is string => !!x).sort();
	return v.at(-1) ?? null;
}
