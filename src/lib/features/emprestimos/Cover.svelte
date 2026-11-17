<script lang="ts">
	// Capa gerada a partir do título (sem baixar imagens). Decorativa.
	import { coverFor } from './loans.ts';

	let { title, author }: { title: string; author?: string | null } = $props();
	const c = $derived(coverFor(title));
	const uid = $props.id();
</script>

<svg class="cover" viewBox="0 0 120 170" aria-hidden="true" focusable="false">
	<defs>
		<pattern id={`p-${uid}`} width="12" height="12" patternUnits="userSpaceOnUse">
			{#if c.pattern === 0}
				<path d="M0 12 L12 0" stroke="#000" stroke-opacity=".18" stroke-width="2" />
			{:else if c.pattern === 1}
				<circle cx="6" cy="6" r="2" fill="#000" fill-opacity=".18" />
			{:else}
				<path d="M0 6 H12" stroke="#000" stroke-opacity=".18" stroke-width="2" />
			{/if}
		</pattern>
	</defs>
	<rect width="120" height="170" fill={`hsl(${c.hue} 55% 62%)`} />
	<rect width="120" height="170" fill={`url(#p-${uid})`} />
	<rect x="0" y="0" width="12" height="170" fill="#000" fill-opacity=".28" />
	<rect x="22" y="34" width="86" height="64" fill="#fffdf6" stroke="#111" stroke-width="3" />
	<text x="65" y="78" text-anchor="middle" class="cover__initials">{c.initials}</text>
	{#if author}
		<text x="65" y="140" text-anchor="middle" class="cover__author">{author.split(/\s+/).at(-1)}</text>
	{/if}
</svg>

<style>
	.cover {
		display: block;
		width: 7.5rem;
		height: auto;
		border: var(--border);
		box-shadow: var(--shadow-hard);
		flex: none;
	}
	.cover__initials {
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		font-size: 30px;
		fill: #111;
	}
	.cover__author {
		font-family: var(--font-mono);
		font-size: 11px;
		fill: #111;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
</style>
