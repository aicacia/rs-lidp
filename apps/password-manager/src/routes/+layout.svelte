<script lang="ts">
	import '../app.css';

	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import type { LayoutProps } from './$types';
	import { getTheme } from '@aicacia/svelte-headless';
	import Notifications from '$lib/common/components/Notifications.svelte';

	let { children }: LayoutProps = $props();

	$effect(() => {
		if (getTheme() === 'dark') {
			document.body.classList.add('dark');
		} else {
			document.body.classList.remove('dark');
		}
	});

	onMount(() => {
		document.body.classList.add('hydrated');
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
<Notifications />