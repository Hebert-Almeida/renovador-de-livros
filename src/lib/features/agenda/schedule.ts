// Regras puras da tela de agenda (5.6). Horários em America/Sao_Paulo, convertidos pelo Intl
// (respeita qualquer mudança futura de fuso ou horário de verão).
import type { Schedule, ScheduleStatus } from '#lib/ipc/bindings.ts';

/** Janela permitida, hora local [início, fim) — espelha `guard::limits::AUTO_WINDOW_*`. */
export const WINDOW_START_HOUR = 7;
export const WINDOW_END_HOUR = 21;
export const TIME_ZONE = 'America/Sao_Paulo';

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

const partsFmt = new Intl.DateTimeFormat('en-US', {
	timeZone: TIME_ZONE,
	hourCycle: 'h23',
	year: 'numeric',
	month: '2-digit',
	day: '2-digit',
	hour: '2-digit',
	minute: '2-digit',
	second: '2-digit'
});

/** Campos de data e hora de um instante no fuso de São Paulo. */
function zonedParts(ms: number) {
	const get = (type: string) => Number(partsFmt.formatToParts(ms).find((p) => p.type === type)?.value);
	return { y: get('year'), mo: get('month'), d: get('day'), h: get('hour'), mi: get('minute'), s: get('second') };
}

/** Diferença (ms) entre o relógio de São Paulo e o UTC no instante dado. */
function zoneOffset(ms: number): number {
	const p = zonedParts(ms);
	return Date.UTC(p.y, p.mo - 1, p.d, p.h, p.mi, p.s) - Math.floor(ms / 1000) * 1000;
}

/** ISO UTC → valor de `<input type="datetime-local">` no horário de São Paulo. */
export function toLocalInput(iso: string): string {
	const p = zonedParts(new Date(iso).getTime());
	return `${p.y}-${pad(p.mo)}-${pad(p.d)}T${pad(p.h)}:${pad(p.mi)}`;
}

/** Valor `YYYY-MM-DDTHH:mm` (São Paulo) → ISO UTC, ou null se inválido. */
export function fromLocalInput(value: string): string | null {
	if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(value)) return null;
	const asUtc = new Date(`${value}:00Z`).getTime();
	if (Number.isNaN(asUtc)) return null;
	// Duas passadas acertam o deslocamento perto de uma troca de fuso.
	let ms = asUtc - zoneOffset(asUtc);
	ms = asUtc - zoneOffset(ms);
	return new Date(ms).toISOString();
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
