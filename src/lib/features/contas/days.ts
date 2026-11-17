// Antecedências por conta (seção 5.4). Limites iguais aos CHECK do banco.
export const RENEW_DAYS = { min: 0, max: 3, default: 1 } as const;
export const REMIND_DAYS = { min: 0, max: 7, default: 2 } as const;

export function range(min: number, max: number): number[] {
	return Array.from({ length: max - min + 1 }, (_, i) => min + i);
}

/** Converte o valor de um <select> e garante que fica dentro dos limites. */
export function clampDays(value: string, limits: { min: number; max: number; default: number }): number {
	const n = Number.parseInt(value, 10);
	if (Number.isNaN(n)) return limits.default;
	return Math.min(limits.max, Math.max(limits.min, n));
}
