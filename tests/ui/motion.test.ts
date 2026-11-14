import { describe, it, expect } from 'vitest';
import { prefersReducedMotion, stamp } from '#lib/motion/index.ts';

describe('motion', () => {
	it('sem matchMedia, assume movimento reduzido', () => {
		const orig = window.matchMedia;
		// @ts-expect-error teste
		window.matchMedia = undefined;
		expect(prefersReducedMotion()).toBe(true);
		window.matchMedia = orig;
	});
	it('stamp resolve', async () => {
		await expect(stamp(document.body)).resolves.toBeUndefined();
	});
});
