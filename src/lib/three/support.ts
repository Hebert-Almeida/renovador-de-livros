// Detecção de capacidade para a cena decorativa do herói.

export function hasWebGL(): boolean {
	if (typeof document === 'undefined') return false;
	try {
		const canvas = document.createElement('canvas');
		const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
		return gl != null;
	} catch {
		return false;
	}
}

export const MAX_DPR = 1.5;

export function clampDpr(dpr: number | undefined): number {
	if (!dpr || !Number.isFinite(dpr) || dpr <= 0) return 1;
	return Math.min(dpr, MAX_DPR);
}
