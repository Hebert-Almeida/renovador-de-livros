<script lang="ts">
	// Vitrine do design system — disponível apenas em desenvolvimento (`vite dev`).
	import { onMount } from 'svelte';
	import Button from '#lib/components/Button.svelte';
	import IconButton from '#lib/components/IconButton.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import Stamp from '#lib/components/Stamp.svelte';
	import Spine from '#lib/components/Spine.svelte';
	import TextField from '#lib/components/TextField.svelte';
	import PasswordField from '#lib/components/PasswordField.svelte';
	import Toggle from '#lib/components/Toggle.svelte';
	import Select from '#lib/components/Select.svelte';
	import Dialog from '#lib/components/Dialog.svelte';
	import Tabs from '#lib/components/Tabs.svelte';
	import DataTable from '#lib/components/DataTable.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import StatusPill from '#lib/components/StatusPill.svelte';
	import ProgressSteps from '#lib/components/ProgressSteps.svelte';
	import { toast } from '#lib/components/toast.svelte.ts';
	import { reveal } from '#lib/motion/index.ts';

	const isDev = import.meta.env.DEV;
	const themes = [
		{ value: 'light', label: 'Claro' },
		{ value: 'dark', label: 'Escuro' },
		{ value: 'contrast', label: 'Alto contraste' }
	];
	let theme = $state('light');
	let user = $state('');
	let pass = $state('');
	let auto = $state(true);
	let dialogOpen = $state(false);
	let sections: HTMLElement | undefined = $state();

	const rows = [
		{ id: 1, titulo: 'Dom Casmurro', autor: 'Machado de Assis', devolucao: '2026-11-20', renovacoes: 2 },
		{ id: 2, titulo: 'Vidas Secas', autor: 'Graciliano Ramos', devolucao: '2026-11-18', renovacoes: 0 },
		{ id: 3, titulo: 'Grande Sertão: Veredas', autor: 'Guimarães Rosa', devolucao: '2026-12-02', renovacoes: 5 }
	];

	onMount(() => {
		if (matchMedia('(prefers-color-scheme: dark)').matches) theme = 'dark';
		const q = new URLSearchParams(location.search).get('tema');
		if (q && themes.some((t) => t.value === q)) theme = q;
		if (sections) reveal([...sections.querySelectorAll('.kit-section')]);
	});
	$effect(() => {
		const root = document.documentElement;
		const previous = root.dataset.theme;
		root.dataset.theme = theme;
		return () => {
			if (previous === undefined) delete root.dataset.theme;
			else root.dataset.theme = previous;
		};
	});
</script>

<h1>Kit de componentes</h1>

{#if !isDev}
	<p>Esta página só existe em desenvolvimento.</p>
{:else}
	<div class="kit" bind:this={sections}>
		<section class="kit-section" aria-labelledby="k-tema">
			<h2 id="k-tema">Tema</h2>
			<Select label="Tema de cores" options={themes} bind:value={theme} />
		</section>

		<section class="kit-section" aria-labelledby="k-botoes">
			<h2 id="k-botoes">Botões</h2>
			<div class="row">
				<Button variant="primary" onclick={() => toast('Empréstimo renovado.', 'ok')}>Renovar agora</Button>
				<Button>Secundário</Button>
				<Button variant="ghost">Discreto</Button>
				<Button variant="danger">Remover conta</Button>
				<Button loading>Renovando…</Button>
				<IconButton label="Atualizar">↻</IconButton>
			</div>
		</section>

		<section class="kit-section" aria-labelledby="k-ficha">
			<h2 id="k-ficha">Ficha, carimbo e lombadas</h2>
			<div class="row">
				<Ficha title="Dom Casmurro" subtitle="Machado de Assis · Tombo 004512" headingLevel={3}>
					<p>Devolução prevista para <time datetime="2026-11-20">20/11/2026</time>.</p>
					{#snippet stamps()}
						<Stamp date="2026-10-30" />
						<Stamp date="2026-11-06" label="Renovado" />
					{/snippet}
					{#snippet actions()}
						<Button variant="primary">Renovar</Button>
						<Button variant="ghost">Detalhes</Button>
					{/snippet}
				</Ficha>
				<div class="row" aria-label="Estante" role="group">
					{#each rows as r (r.id)}
						<Spine title={r.titulo} author={r.autor} due={r.devolucao} href={`#livro-${r.id}`} />
					{/each}
				</div>
			</div>
		</section>

		<section class="kit-section" aria-labelledby="k-campos">
			<h2 id="k-campos">Campos</h2>
			<div class="grid">
				<TextField label="Matrícula" hint="Somente números." bind:value={user} inputmode="numeric" autocomplete="username" />
				<PasswordField label="Senha da biblioteca" bind:value={pass} />
				<TextField label="E-mail" type="email" error="Informe um e-mail válido." />
				<Toggle label="Renovação automática" description="Renova sozinho dois dias antes do prazo." bind:checked={auto} />
			</div>
		</section>

		<section class="kit-section" aria-labelledby="k-estado">
			<h2 id="k-estado">Estado e progresso</h2>
			<div class="row">
				<StatusPill status="ok" label="Em dia" />
				<StatusPill status="warn" label="Vence amanhã" />
				<StatusPill status="danger" label="Atrasado" />
				<StatusPill status="info" label="Reservado" />
				<StatusPill label="Sem dados" />
			</div>
			<ProgressSteps
				label="Ciclo de renovação"
				steps={[
					{ id: 'login', label: 'Entrar', state: 'done' },
					{ id: 'list', label: 'Listar empréstimos', state: 'active' },
					{ id: 'renew', label: 'Renovar', state: 'pending' }
				]}
			/>
			<Skeleton lines={3} />
		</section>

		<section class="kit-section" aria-labelledby="k-abas">
			<h2 id="k-abas">Abas e tabela</h2>
			<Tabs
				label="Visões"
				tabs={[
					{ id: 'tabela', label: 'Tabela' },
					{ id: 'vazio', label: 'Vazio' }
				]}
			>
				{#snippet panel(id)}
					{#if id === 'tabela'}
						<DataTable
							caption="Empréstimos ativos"
							rowKey="id"
							{rows}
							columns={[
								{ key: 'titulo', label: 'Título', sortable: true },
								{ key: 'autor', label: 'Autor' },
								{ key: 'devolucao', label: 'Devolução', sortable: true, mono: true },
								{ key: 'renovacoes', label: 'Renovações', sortable: true, numeric: true }
							]}
						/>
					{:else}
						<EmptyState title="Nenhum empréstimo" description="Quando você pegar um livro, ele aparece aqui." headingLevel={3}>
							{#snippet actions()}<Button>Sincronizar</Button>{/snippet}
						</EmptyState>
					{/if}
				{/snippet}
			</Tabs>
		</section>

		<section class="kit-section" aria-labelledby="k-dialogo">
			<h2 id="k-dialogo">Diálogo e avisos</h2>
			<div class="row">
				<Button onclick={() => (dialogOpen = true)}>Abrir diálogo</Button>
				<Button onclick={() => toast('Não foi possível conectar.', 'danger')}>Mostrar erro</Button>
			</div>
			<Dialog bind:open={dialogOpen} title="Remover conta?">
				<p>A senha guardada será apagada deste dispositivo.</p>
				{#snippet actions()}
					<Button onclick={() => (dialogOpen = false)}>Cancelar</Button>
					<Button variant="danger" onclick={() => (dialogOpen = false)}>Remover</Button>
				{/snippet}
			</Dialog>
		</section>
	</div>
{/if}

<style>
	.kit {
		display: grid;
		gap: var(--space-5);
	}
	.kit-section {
		display: grid;
		gap: var(--space-3);
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: var(--space-4);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
		gap: var(--space-4);
	}
</style>
