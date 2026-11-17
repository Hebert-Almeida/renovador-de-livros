// Regras de apresentação dos empréstimos (puras, testáveis sem DOM).
import type { LoanStatus, LoanView_Serialize, RenewalEvent_Serialize } from '#lib/ipc/index.ts';
import type { Status } from '#lib/components/StatusPill.svelte';
import t from '#lib/i18n/pt-BR/emprestimos.ts';

/** `AAAA-MM-DD…` → `DD/MM/AAAA`. */
export function formatDate(iso: string | null | undefined): string {
	if (!iso) return '—';
	const [y, m, d] = iso.slice(0, 10).split('-');
	return d && m && y ? `${d}/${m}/${y}` : iso;
}

/** Dias inteiros de `today` até `due` (negativo = atrasado). */
export function daysUntil(due: string, today: Date = new Date()): number {
	const [y, m, d] = due.slice(0, 10).split('-').map(Number);
	const a = Date.UTC(y, m - 1, d);
	const b = Date.UTC(today.getFullYear(), today.getMonth(), today.getDate());
	return Math.round((a - b) / 86_400_000);
}

export interface Pill {
	status: Status;
	label: string;
}

export function loanPill(v: LoanView_Serialize, today: Date = new Date()): Pill {
	const s: LoanStatus = v.loan.status;
	if (s === 'returned') return { status: 'neutral', label: t.statusReturned };
	const d = daysUntil(v.loan.due_date, today);
	if (s === 'overdue' || d < 0) return { status: 'danger', label: t.daysLeft(Math.min(d, -1)) };
	if (d <= 2) return { status: 'warn', label: t.daysLeft(d) };
	return { status: 'ok', label: t.daysLeft(d) };
}

export function eventLabel(e: RenewalEvent_Serialize): string {
	return t.result[e.result] ?? e.result;
}

/** Carimbos em ordem cronológica. */
export function sortEvents(events: RenewalEvent_Serialize[]): RenewalEvent_Serialize[] {
	return [...events].sort((a, b) => a.attempted_at.localeCompare(b.attempted_at));
}

/** Mensagem legível para um erro do IPC (`{ kind, message }`). */
export function errorMessage(e: unknown): string {
	if (e && typeof e === 'object' && 'kind' in e) {
		const err = e as { kind: string; message?: unknown };
		if (typeof err.message === 'string' && err.message) return `${t.actionError} (${err.message})`;
		return `${t.actionError} (${err.kind})`;
	}
	return t.actionError;
}

/** Capa gerada a partir do título: matiz, padrão e iniciais determinísticos. */
export interface CoverSpec {
	hue: number;
	pattern: 0 | 1 | 2;
	initials: string;
}

export function hashString(s: string): number {
	let h = 2166136261;
	for (const ch of s) {
		h ^= ch.codePointAt(0)!;
		h = Math.imul(h, 16777619);
	}
	return h >>> 0;
}

export function coverFor(title: string): CoverSpec {
	const h = hashString(title);
	const words = title
		.split(/\s+/)
		.map((w) => w.replace(/[^\p{L}\p{N}]/gu, ''))
		.filter((w) => w.length > 2 || /^\p{Lu}/u.test(w));
	const initials = (words.slice(0, 2).map((w) => w[0]).join('') || title.slice(0, 1) || '?').toUpperCase();
	return { hue: h % 360, pattern: ((h >>> 9) % 3) as 0 | 1 | 2, initials };
}
