<script lang="ts">
	import { onMount } from 'svelte';

	import '../app.css';

	import { getTheme } from '@aicacia/svelte-headless';
	import favicon from '$lib/assets/favicon.svg';
	import Notifications from '$lib/common/components/Notifications.svelte';
	import { loadStorageBridgeConfig } from '$lib/common/state/storageConfig.svelte';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	function consume(..._values: unknown[]): void {}

	$effect(() => {
		consume(children, Notifications, favicon);
	});

	$effect(() => {
		if (getTheme() === 'dark') {
			document.body.classList.add('dark');
		} else {
			document.body.classList.remove('dark');
		}
	});

	onMount(() => {
		document.body.classList.add('hydrated');

		void loadStorageBridgeConfig();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
<Notifications />
