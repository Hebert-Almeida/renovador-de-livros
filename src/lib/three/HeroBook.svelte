<script lang="ts">
	import { onMount } from 'svelte';
	import { prefersReducedMotion } from '#lib/motion/index.ts';
	import { hasWebGL } from './support.ts';
	import type { HeroScene } from './scene.ts';

	let { class: className = '' }: { class?: string } = $props();

	let canvas: HTMLCanvasElement | undefined = $state();
	let mode: 'fallback' | 'webgl' = $state('fallback');

	onMount(() => {
		if (prefersReducedMotion() || !hasWebGL()) return;
		mode = 'webgl';
		let hero: HeroScene | undefined;
		let disposed = false;
		let resizeObs: ResizeObserver | undefined;
		let io: IntersectionObserver | undefined;
		let onScreen = true;

		const update = () => hero?.setActive(!document.hidden && onScreen);
		document.addEventListener('visibilitychange', update);

		import('./scene.ts')
			.then(({ createHeroScene }) => {
				if (disposed || !canvas) return;
				const el = canvas;
				hero = createHeroScene(el);
				hero.resize(el.clientWidth, el.clientHeight);
				if (typeof ResizeObserver !== 'undefined') {
					resizeObs = new ResizeObserver(() => hero?.resize(el.clientWidth, el.clientHeight));
					resizeObs.observe(el);
				}
				if (typeof IntersectionObserver !== 'undefined') {
					io = new IntersectionObserver(([entry]) => {
						onScreen = entry.isIntersecting;
						update();
					});
					io.observe(el);
				}
				update();
			})
			.catch(() => {
				if (!disposed) mode = 'fallback';
			});

		return () => {
			disposed = true;
			document.removeEventListener('visibilitychange', update);
			resizeObs?.disconnect();
			io?.disconnect();
			hero?.dispose();
		};
	});
</script>

<div class="hero-book {className}" aria-hidden="true" data-mode={mode}>
	{#if mode === 'webgl'}
		<canvas bind:this={canvas}></canvas>
	{:else}
		<svg viewBox="0 0 200 240" focusable="false" data-testid="hero-book-fallback">
			<rect x="46" y="36" width="126" height="176" fill="#f4f1ea" stroke="#111" stroke-width="4" />
			<rect x="40" y="22" width="126" height="184" fill="#111" stroke="#111" stroke-width="4" />
			<rect x="40" y="22" width="14" height="184" fill="#f4f1ea" stroke="#111" stroke-width="4" />
			<rect x="68" y="60" width="80" height="10" fill="#f4f1ea" />
			<rect x="68" y="78" width="56" height="6" fill="#f4f1ea" />
		</svg>
	{/if}
</div>

<style>
	.hero-book {
		aspect-ratio: 5 / 6;
		width: 100%;
		border: 3px solid #111;
		background: #f4f1ea;
		box-shadow: 6px 6px 0 #111;
	}
	canvas,
	svg {
		display: block;
		width: 100%;
		height: 100%;
	}
</style>
