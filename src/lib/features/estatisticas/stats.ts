import type { DateRange, MonthlyStat } from '#lib/ipc/bindings.ts';

export type PeriodKey = '6m' | '12m' | 'all';

const pad = (n: number) => String(n).padStart(2, '0');
export const isoDate = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** Intervalo inclusivo (YYYY-MM-DD) para o período, começando no 1º dia do mês. */
export function rangeFor(period: PeriodKey, today = new Date()): DateRange {
	const to = isoDate(today);
	if (period === 'all') return { from: '2000-01-01', to };
	const months = period === '6m' ? 6 : 12;
	const from = new Date(today.getFullYear(), today.getMonth() - (months - 1), 1);
	return { from: isoDate(from), to };
}

/** Preenche meses sem dados com zero para o eixo não pular meses. */
export function fillMonths(stats: MonthlyStat[]): MonthlyStat[] {
	if (stats.length === 0) return [];
	const sorted = [...stats].sort((a, b) => a.month.localeCompare(b.month));
	const byMonth = new Map(sorted.map((s) => [s.month, s]));
	const [y0, m0] = sorted[0].month.split('-').map(Number);
	const last = sorted[sorted.length - 1].month;
	const out: MonthlyStat[] = [];
	for (let y = y0, m = m0; out.length < 1200; m++) {
		if (m > 12) {
			m = 1;
			y++;
		}
		const key = `${y}-${pad(m)}`;
		out.push(byMonth.get(key) ?? { month: key, renewals: 0, successes: 0, failures: 0 });
		if (key === last) break;
	}
	return out;
}

const monthFmt = new Intl.DateTimeFormat('pt-BR', { month: 'short', year: 'numeric', timeZone: 'UTC' });
const monthShortFmt = new Intl.DateTimeFormat('pt-BR', { month: 'short', timeZone: 'UTC' });
const toDate = (ym: string) => {
	const [y, m] = ym.split('-').map(Number);
	return new Date(Date.UTC(y, m - 1, 1));
};
export const monthLabel = (ym: string) => monthFmt.format(toDate(ym)).replace('.', '');
export const monthShort = (ym: string) => monthShortFmt.format(toDate(ym)).replace('.', '');

const pctFmt = new Intl.NumberFormat('pt-BR', { style: 'percent', maximumFractionDigits: 0 });
const intFmt = new Intl.NumberFormat('pt-BR');
/** `rate` é uma fração 0..1. */
export const formatPercent = (rate: number | null | undefined) => (rate == null ? null : pctFmt.format(rate));
export const formatInt = (n: number) => intFmt.format(n);

/** Taxa de sucesso de um mês (null sem tentativas). */
export function monthRate(s: MonthlyStat): number | null {
	const tries = s.successes + s.failures;
	return tries === 0 ? null : s.successes / tries;
}

/** Topo "redondo" do eixo y e marcas (no máx. 5). */
export function niceScale(max: number): { top: number; ticks: number[] } {
	if (max <= 0) return { top: 1, ticks: [0, 1] };
	const rough = max / 4;
	const mag = 10 ** Math.floor(Math.log10(rough));
	const step = [1, 2, 5, 10].map((f) => f * mag).find((s) => s >= rough) ?? 10 * mag;
	const stepInt = Math.max(1, Math.round(step));
	const top = Math.ceil(max / stepInt) * stepInt;
	const ticks: number[] = [];
	for (let v = 0; v <= top; v += stepInt) ticks.push(v);
	return { top, ticks };
}
