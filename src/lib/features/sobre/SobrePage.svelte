<script lang="ts">
	import { onMount } from 'svelte';
	import Button from '#lib/components/Button.svelte';
	import Ficha from '#lib/components/Ficha.svelte';
	import Tabs from '#lib/components/Tabs.svelte';
	import DataTable from '#lib/components/DataTable.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Skeleton from '#lib/components/Skeleton.svelte';
	import { commands } from '#lib/ipc/index.ts';
	import { reveal } from '#lib/motion/index.ts';
	import t from '#lib/i18n/pt-BR/sobre.ts';
	import { thirdPartyLicenses } from './licenses.ts';
	import { toRows, DIAGNOSTICS_LIMIT, type DiagnosticRow } from './diagnostics.ts';

	let version: string | null = $state(null);
	let versionFailed = $state(false);
	let rows: DiagnosticRow[] = $state([]);
	let diagState: 'loading' | 'ready' | 'error' = $state('loading');
	let announce = $state('');
	let root: HTMLElement | undefined = $state();

	async function loadVersion() {
		try {
			version = (await commands.appStatus()).version;
		} catch {
			versionFailed = true;
		}
	}

	async function loadDiagnostics() {
		diagState = 'loading';
		announce = '';
		try {
			rows = toRows(await commands.diagnosticsRequests(DIAGNOSTICS_LIMIT));
			diagState = 'ready';
			announce = t.diagnostics.loaded(rows.length);
		} catch {
			diagState = 'error';
			announce = t.diagnostics.error;
		}
	}

	onMount(() => {
		if (root) reveal([...root.querySelectorAll('.sobre-section')]);
		loadVersion();
		loadDiagnostics();
	});
</script>

<div class="sobre" bind:this={root}>
	<header class="sobre-section">
		<h1>{t.title}</h1>
		<p class="lead">{t.intro}</p>
	</header>

	<section class="sobre-section" aria-labelledby="sobre-versao">
		<h2 id="sobre-versao">{t.version.heading}</h2>
		<Ficha title="Renovador de Livros" subtitle={t.version.license} headingLevel={3}>
			<p>
				{t.version.label}:
				{#if version}
					<span class="mono" data-testid="app-version">{version}</span>
				{:else if versionFailed}
					<span data-testid="app-version">{t.version.unavailable}</span>
				{:else}
					<span>{t.version.loading}</span>
				{/if}
			</p>
			<p>{t.version.disclaimer}</p>
		</Ficha>
	</section>

	<section class="sobre-section" aria-labelledby="sobre-legal">
		<h2 id="sobre-legal">{t.legal.heading}</h2>
		<Tabs
			label={t.legal.tabsLabel}
			tabs={[
				{ id: 'privacy', label: t.legal.privacyTab },
				{ id: 'usage', label: t.legal.usageTab }
			]}
		>
			{#snippet panel(id)}
				{@const doc = id === 'privacy' ? t.legal.privacy : t.legal.usage}
				<p>{doc.intro}</p>
				<ul>
					{#each doc.points as p (p)}<li>{p}</li>{/each}
				</ul>
				<p class="source">{doc.source}</p>
			{/snippet}
		</Tabs>
	</section>

	<section class="sobre-section" aria-labelledby="sobre-licencas">
		<h2 id="sobre-licencas">{t.licenses.heading}</h2>
		<DataTable
			caption={t.licenses.caption}
			rowKey="name"
			rows={thirdPartyLicenses}
			columns={[
				{ key: 'name', label: t.licenses.name, sortable: true, mono: true },
				{ key: 'version', label: t.licenses.versionCol, mono: true },
				{ key: 'license', label: t.licenses.license, sortable: true }
			]}
		/>
		<p class="source">{t.licenses.note}</p>
	</section>

	<section class="sobre-section" aria-labelledby="sobre-diag">
		<h2 id="sobre-diag">{t.diagnostics.heading}</h2>
		<p>{t.diagnostics.description}</p>
		<div>
			<Button onclick={loadDiagnostics} loading={diagState === 'loading'}>{t.diagnostics.refresh}</Button>
		</div>
		<p class="visually-hidden" role="status" aria-live="polite">{announce}</p>
		{#if diagState === 'loading'}
			<Skeleton lines={3} />
		{:else if diagState === 'error'}
			<p role="alert">{t.diagnostics.error}</p>
		{:else if rows.length === 0}
			<EmptyState title={t.diagnostics.empty} headingLevel={3} />
		{:else}
			<DataTable
				caption={t.diagnostics.caption}
				rowKey="id"
				{rows}
				columns={[
					{ key: 'at', label: t.diagnostics.at, mono: true },
					{ key: 'host', label: t.diagnostics.host, mono: true },
					{ key: 'kind', label: t.diagnostics.kind },
					{ key: 'status', label: t.diagnostics.status, numeric: true, mono: true },
					{ key: 'outcome', label: t.diagnostics.outcome },
					{ key: 'account', label: t.diagnostics.account }
				]}
			/>
		{/if}
	</section>
</div>

<style>
	.sobre {
		display: grid;
		gap: var(--space-5);
	}
	.sobre-section {
		display: grid;
		gap: var(--space-3);
	}
	.lead {
		max-width: 60ch;
	}
	.mono {
		font-family: var(--font-mono);
	}
	.source {
		font-size: 0.9em;
	}
	ul {
		display: grid;
		gap: var(--space-2, 0.5rem);
		padding-left: 1.25rem;
		max-width: 70ch;
	}
</style>
