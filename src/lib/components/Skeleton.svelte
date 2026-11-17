<script lang="ts">
	let {
		lines = 3,
		label = 'Carregando…',
		height = '1rem'
	}: { lines?: number; label?: string; height?: string } = $props();
</script>

<div class="skeleton" role="status" aria-busy="true">
	<span class="visually-hidden">{label}</span>
	{#each Array.from({ length: lines }, (_, i) => i) as i (i)}
		<span class="skeleton__line" aria-hidden="true" style:height style:width={i === lines - 1 && lines > 1 ? '60%' : '100%'}></span>
	{/each}
</div>

<style>
	.skeleton {
		display: grid;
		gap: var(--space-2);
	}
	.skeleton__line {
		display: block;
		background: linear-gradient(
			90deg,
			var(--color-skeleton) 0%,
			var(--color-skeleton-shine) 50%,
			var(--color-skeleton) 100%
		);
		background-size: 200% 100%;
		border: var(--border-width-thin) solid var(--color-rule);
		animation: shimmer 1.4s linear infinite;
	}
	@keyframes shimmer {
		from {
			background-position: 200% 0;
		}
		to {
			background-position: -200% 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		:global(:root:not([data-motion='full'])) .skeleton__line {
			animation: none;
		}
	}
	:global(:root[data-motion='reduced']) .skeleton__line {
		animation: none;
	}
</style>
