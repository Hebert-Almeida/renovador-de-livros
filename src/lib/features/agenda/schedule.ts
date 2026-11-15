// Regras puras da tela de agenda (5.6). Horários em America/Sao_Paulo (UTC−03:00, sem horário de verão desde 2019).
import type { Schedule, ScheduleStatus } from '#lib/ipc/bindings.ts';

/** Janela permitida, hora local [início, fim) — espelha `guard::limits::AUTO_WINDOW_*`. */
export const WINDOW_START_HOUR = 7;
export const WINDOW_END_HOUR = 21;
export const TIME_ZONE = 'America/Sao_Paulo';
const OFFSET = '-03:00';

export type Action = 'pause' | 'reactivate' | 'reschedule' | 'cancel';

const ACTIONS: Record<ScheduleStatus, Action[]> = {
	pending: ['pause', 'reschedule', 'cancel'],
	paused: ['reactivate', 'reschedule', 'cancel'],
	cancelled: ['reactivate'],
	failed: ['reschedule'],
	done: []
};

export function actionsFor(status: ScheduleStatus): Action[] {
	return ACTIONS[status];
}

const pad = (n: number) => String(n).padStart(2, '0');

/** ISO UTC → valor de `<input type="datetime-local">` no horário de São Paulo. */
export function toLocalInput(iso: string): string {
	const d = new Date(new Date(iso).getTime() - 3 * 3600_000);
	return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}T${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;
}

/** Valor `YYYY-MM-DDTHH:mm` (São Paulo) → ISO UTC, ou null se inválido. */
export function fromLocalInput(value: string): string | null {
	if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(value)) return null;
	const d = new Date(`${value}:00${OFFSET}`);
	return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

export type RescheduleError = 'invalid' | 'past' | 'window';

/** Valida um novo horário: precisa ser futuro e dentro da janela 07:00–21:00 (21:00 excluso). */
export function validateRunAt(value: string, now: Date = new Date()): RescheduleError | null {
	const iso = fromLocalInput(value);
	if (!iso) return 'invalid';
	if (new Date(iso).getTime() <= now.getTime()) return 'past';
	const hour = Number(value.slice(11, 13));
	if (hour < WINDOW_START_HOUR || hour >= WINDOW_END_HOUR) return 'window';
	return null;
}

const fmt = new Intl.DateTimeFormat('pt-BR', { timeZone: TIME_ZONE, dateStyle: 'short', timeStyle: 'short' });

export function formatRunAt(iso: string): string {
	return fmt.format(new Date(iso));
}

export function sortByRunAt(list: Schedule[]): Schedule[] {
	return [...list].sort((a, b) => a.run_at.localeCompare(b.run_at) || a.id - b.id);
}
