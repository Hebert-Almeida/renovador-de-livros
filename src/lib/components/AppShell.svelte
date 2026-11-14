<script lang="ts">
	// Stub da Onda 0 — implementação real na W1-07.
	import type { Snippet } from 'svelte';
	import { routes } from '#lib/routes.ts';
	let { children }: { children: Snippet } = $props();
</script>

<a class="skip-link" href="#conteudo">Pular para o conteúdo</a>
<header>
	<nav aria-label="Principal">
		<ul>
			{#each routes as r (r.href)}
				<li><a href={r.href}>{r.label}</a></li>
			{/each}
		</ul>
	</nav>
</header>
<main id="conteudo" tabindex="-1">
	{@render children()}
</main>
