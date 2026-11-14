// API de movimento. GSAP para timelines/transições/carimbo; anime.js para micro-interações.
// Toda animação da UI deve passar por aqui, respeitando prefers-reduced-motion.
import { gsap } from 'gsap';
import { animate } from 'animejs';

export function prefersReducedMotion(): boolean {
	if (typeof window === 'undefined' || !window.matchMedia) return true;
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export interface MotionOptions {
	/** Atraso em ms. */
	delay?: number;
	/** Duração em ms. */
	duration?: number;
}

const ms = (v: number | undefined, d: number) => (v ?? d) / 1000;

/** Revela elementos ao entrar na tela (anime.js, escalonado). */
export function reveal(targets: Element | Element[], opts: MotionOptions = {}): void {
	const list = Array.isArray(targets) ? targets : [targets];
	if (list.length === 0 || prefersReducedMotion()) return;
	list.forEach((el, i) => {
		animate(el as HTMLElement, {
			opacity: [0, 1],
			translateY: ['0.75rem', '0rem'],
			duration: opts.duration ?? 360,
			delay: (opts.delay ?? 0) + i * 60,
			ease: 'outQuad',
			onComplete: () => (el as HTMLElement).style.removeProperty('transform')
		});
	});
}

/** Animação de carimbo de data sobre uma ficha (GSAP). */
export function stamp(target: Element, opts: MotionOptions = {}): Promise<void> {
	if (prefersReducedMotion()) return Promise.resolve();
	return new Promise((resolve) => {
		gsap.fromTo(
			target,
			{ scale: 1.8, rotation: -14, opacity: 0 },
			{
				scale: 1,
				rotation: -6,
				opacity: 1,
				duration: ms(opts.duration, 280),
				delay: ms(opts.delay, 0),
				ease: 'back.out(2.4)',
				clearProps: 'scale,opacity',
				onComplete: () => resolve()
			}
		);
	});
}

/** Transição entre páginas (GSAP). */
export function pageTransition(from: Element | null, to: Element, opts: MotionOptions = {}): Promise<void> {
	if (prefersReducedMotion()) return Promise.resolve();
	const d = ms(opts.duration, 240);
	return new Promise((resolve) => {
		const tl = gsap.timeline({ delay: ms(opts.delay, 0), onComplete: () => resolve() });
		if (from) tl.to(from, { opacity: 0, x: -16, duration: d / 2, ease: 'power2.in' });
		tl.fromTo(to, { opacity: 0, x: 16 }, { opacity: 1, x: 0, duration: d, ease: 'power2.out', clearProps: 'opacity,transform' });
	});
}

/** Micro-interação de "pressionar" (anime.js). */
export function press(target: Element): void {
	if (prefersReducedMotion()) return;
	const el = target as HTMLElement;
	// Remove o transform inline ao fim para não anular os estados :hover/:active do CSS.
	animate(el, {
		scale: [0.96, 1],
		duration: 160,
		ease: 'outQuad',
		onComplete: () => el.style.removeProperty('transform')
	});
}
