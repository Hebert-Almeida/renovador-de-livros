import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import HeroBook from '#lib/three/HeroBook.svelte';
import { clampDpr, hasWebGL } from '#lib/three/support.ts';

const origMatchMedia = window.matchMedia;

function mockReducedMotion(reduce: boolean) {
	window.matchMedia = ((q: string) => ({ matches: reduce, media: q })) as never;
}

afterEach(() => {
	cleanup();
	vi.restoreAllMocks();
	window.matchMedia = origMatchMedia;
});

describe('HeroBook', () => {
	it('sem WebGL, mostra o SVG estático e fica oculto para leitores de tela', () => {
		mockReducedMotion(false);
		vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
		expect(hasWebGL()).toBe(false);
		const { container, getByTestId } = render(HeroBook);
		expect(getByTestId('hero-book-fallback')).toBeInTheDocument();
		expect(container.querySelector('canvas')).toBeNull();
		expect(container.querySelector('.hero-book')).toHaveAttribute('aria-hidden', 'true');
	});

	it('com movimento reduzido, usa o fallback mesmo com WebGL', () => {
		mockReducedMotion(true);
		vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({} as never);
		const { getByTestId } = render(HeroBook);
		expect(getByTestId('hero-book-fallback')).toBeInTheDocument();
	});

	it('limita o DPR a 1,5', () => {
		expect(clampDpr(3)).toBe(1.5);
		expect(clampDpr(1)).toBe(1);
		expect(clampDpr(undefined)).toBe(1);
	});
});
