// API congelada de movimento (Onda 0). Implementação com GSAP/anime.js na W1-07.
// Toda animação da UI deve passar por aqui, respeitando prefers-reduced-motion.

export function prefersReducedMotion(): boolean {
	if (typeof window === 'undefined' || !window.matchMedia) return true;
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export interface MotionOptions {
	delay?: number;
	duration?: number;
}

/** Revela elementos ao entrar na tela. */
export function reveal(_targets: Element | Element[], _opts: MotionOptions = {}): void {}

/** Animação de carimbo de data sobre uma ficha. */
export function stamp(_target: Element, _opts: MotionOptions = {}): Promise<void> {
	return Promise.resolve();
}

/** Transição entre páginas. */
export function pageTransition(_from: Element | null, _to: Element, _opts: MotionOptions = {}): Promise<void> {
	return Promise.resolve();
}
