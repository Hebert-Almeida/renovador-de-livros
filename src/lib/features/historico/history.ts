// Lógica pura da tela de histórico (sem DOM), testável isoladamente.
import { save } from '@tauri-apps/plugin-dialog';
import { commands } from '#lib/ipc/index.ts';
import type { ExportFormat, HistoryFilter_Deserialize, RenewalEvent_Serialize, RenewalResult } from '#lib/ipc/index.ts';
import t from '#lib/i18n/pt-BR/historico.ts';

export const PAGE_SIZE = 25;

export const RESULTS: RenewalResult[] = [
	'success',
	'reserved',
	'limit_reached',
	'blocked',
	'overdue',
	'network_error',
	'parse_error',
	'captcha',
	'rate_limited'
];

export interface FilterForm {
	account: string;
	result: string;
	from: string;
	to: string;
}

export const emptyForm = (): FilterForm => ({ account: '', result: '', from: '', to: '' });

/** Início do dia local (`YYYY-MM-DD`) em ISO UTC. */
export function startOfDay(date: string): string {
	return new Date(`${date}T00:00:00`).toISOString();
}
/** Fim do dia local em ISO UTC. */
export function endOfDay(date: string): string {
	return new Date(`${date}T23:59:59.999`).toISOString();
}

export function toFilter(f: FilterForm): HistoryFilter_Deserialize {
	return {
		account_id: f.account ? Number(f.account) : null,
		loan_id: null,
		result: f.result ? (f.result as RenewalResult) : null,
		from: f.from ? startOfDay(f.from) : null,
		to: f.to ? endOfDay(f.to) : null
	};
}

export function isRangeValid(f: FilterForm): boolean {
	return !f.from || !f.to || f.from <= f.to;
}

const dateTime = new Intl.DateTimeFormat('pt-BR', { dateStyle: 'short', timeStyle: 'short' });
const dateOnly = new Intl.DateTimeFormat('pt-BR', { dateStyle: 'short', timeZone: 'UTC' });

export function formatDateTime(iso: string): string {
	const d = new Date(iso);
	return Number.isNaN(d.getTime()) ? iso : dateTime.format(d);
}

/** Datas de devolução (`YYYY-MM-DD` ou ISO) sem deslocamento de fuso. */
export function formatDate(value: string | null | undefined): string {
	if (!value) return '—';
	const d = new Date(value.length === 10 ? `${value}T00:00:00Z` : value);
	return Number.isNaN(d.getTime()) ? value : dateOnly.format(d);
}

export function dueChange(e: Pick<RenewalEvent_Serialize, 'old_due_date' | 'new_due_date'>): string {
	if (!e.old_due_date && !e.new_due_date) return '—';
	if (!e.new_due_date || e.new_due_date === e.old_due_date) return formatDate(e.old_due_date);
	return `${formatDate(e.old_due_date)} → ${formatDate(e.new_due_date)}`;
}

/**
 * Exporta o histórico: abre o diálogo "salvar" e chama o comando IPC.
 * Retorna o número de linhas exportadas, ou `null` se o usuário cancelou.
 */
export async function exportHistory(format: ExportFormat): Promise<number | null> {
	const path = await save({
		title: t.exportDialogTitle,
		defaultPath: `historico.${format}`,
		filters: [{ name: format.toUpperCase(), extensions: [format] }]
	});
	if (!path) return null;
	return commands.historyExport(format, path);
}
