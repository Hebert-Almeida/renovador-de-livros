<script lang="ts">
	import t from '#lib/i18n/pt-BR/agenda.ts';
	import AgendaView from '#lib/features/agenda/AgendaView.svelte';
</script>

<svelte:head><title>{t.title} · Renovador de Livros</title></svelte:head>
<h1>{t.title}</h1>
<AgendaView />
