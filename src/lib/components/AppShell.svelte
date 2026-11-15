<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { routes } from '#lib/routes.ts';
	import ToastRegion from './ToastRegion.svelte';

	let { children }: { children: Snippet } = $props();

	const path = $derived(page.url?.pathname ?? '/');
	const isCurrent = (href: string) => (href === '/' ? path === '/' : path === href || path.startsWith(href + '/'));
</script>

<a class="skip-link" href="#conteudo">Pular para o conteúdo</a>
<div class="shell">
	<header class="shell__side">
		<p class="shell__brand"><span aria-hidden="true">▤</span> Renovador</p>
		<nav aria-label="Principal">
			<ul class="shell__nav">
				{#each routes as r (r.href)}
					<li>
						<a class="shell__link" href={r.href} aria-current={isCurrent(r.href) ? 'page' : undefined}>{r.label}</a>
					</li>
				{/each}
			</ul>
		</nav>
	</header>
	<main id="conteudo" class="shell__main" tabindex="-1">
		{@render children()}
	</main>
</div>
<ToastRegion />

<style>
	.shell {
		display: grid;
		grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
		min-height: 100vh;
	}
	.shell__side {
		border-right: var(--border);
		background: var(--color-paper-sunken);
		padding: var(--space-4) var(--space-3);
	}
	.shell__brand {
		margin: 0 0 var(--space-4);
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		font-size: var(--text-lg);
		text-transform: uppercase;
		letter-spacing: var(--tracking-caps);
		border-bottom: var(--border);
		padding-bottom: var(--space-2);
	}
	.shell__nav {
		display: grid;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.shell__link {
		display: flex;
		align-items: center;
		min-height: var(--target-min);
		padding: var(--space-1) var(--space-3);
		border: var(--border-width) solid transparent;
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		text-decoration: none;
	}
	.shell__link:hover {
		border-color: var(--color-rule);
	}
	.shell__link[aria-current='page'] {
		border-color: var(--color-rule);
		background: var(--color-paper-raised);
		box-shadow: var(--shadow-hard);
		text-decoration: underline;
		text-decoration-thickness: 0.1875rem;
	}
	.shell__main {
		padding: var(--space-5) var(--space-4);
		max-width: var(--content-max);
		width: 100%;
	}
	.shell__main:focus {
		outline: none;
	}
	@media (max-width: 48rem) {
		.shell {
			grid-template-columns: 1fr;
		}
		.shell__side {
			border-right: 0;
			border-bottom: var(--border);
		}
		.shell__nav {
			display: flex;
			flex-wrap: wrap;
		}
	}
</style>
