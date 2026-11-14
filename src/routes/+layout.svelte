<script lang="ts">
	import '../styles/tokens.css';
	import '../styles/base.css';
	import AppShell from '#lib/components/AppShell.svelte';
	let { children } = $props();
</script>

<AppShell>
	{@render children()}
</AppShell>
