import type { DateRange, MonthlyStat } from '#lib/ipc/bindings.ts';

export type PeriodKey = '6m' | '12m' | 'all';

const pad = (n: number) => String(n).padStart(2, '0');
export const isoDate = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/**
 * Intervalo inclusivo (YYYY-MM-DD) para o período, começando no 1º dia do mês local.
 * O banco grava `attempted_at` em UTC e compara as datas como texto, então o fim é a data
 * UTC de agora: com a data local, uma renovação às 21h30 em São Paulo (00h30 UTC do dia
 * seguinte) ficaria fora do intervalo até o dia virar.
 */
export function rangeFor(period: PeriodKey, today = new Date()): DateRange {
	const to = today.toISOString().slice(0, 10);
	if (period === 'all') return { from: '2000-01-01', to };
	const months = period === '6m' ? 6 : 12;
	const from = new Date(today.getFullYear(), today.getMonth() - (months - 1), 1);
	return { from: isoDate(from), to };
}

/** Mês local (YYYY-MM) de uma data. */
export const monthOf = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}`;

/**
 * Preenche com zero os meses sem dados, de `from` até `to` (YYYY-MM), para o eixo cobrir o
 * período escolhido. Sem `from` (período "desde o início"), começa no primeiro mês com dados.
 * Meses com dados fora dos limites alargam o intervalo em vez de sumir.
 */
export function fillMonths(stats: MonthlyStat[], to: string, from: string | null = null): MonthlyStat[] {
	const sorted = [...stats].sort((a, b) => a.month.localeCompare(b.month));
	if (sorted.length === 0 && from == null) return [];
	const byMonth = new Map(sorted.map((s) => [s.month, s]));
	const first = [from, sorted[0]?.month].filter((m): m is string => m != null).sort()[0];
	const last = [to, sorted[sorted.length - 1]?.month].filter((m): m is string => m != null).sort().at(-1)!;
	const [y0, m0] = first.split('-').map(Number);
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
const pctFmt1 = new Intl.NumberFormat('pt-BR', { style: 'percent', minimumFractionDigits: 1, maximumFractionDigits: 1 });
const intFmt = new Intl.NumberFormat('pt-BR');
/** `rate` é uma fração 0..1. Abaixo de 1, nunca arredonda para "100 %" (199/200 vira "99,5 %"). */
export function formatPercent(rate: number | null | undefined): string | null {
	if (rate == null) return null;
	if (rate < 1 && rate >= 0.995) return pctFmt1.format(Math.floor(rate * 1000) / 1000);
	return pctFmt.format(rate);
}
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
