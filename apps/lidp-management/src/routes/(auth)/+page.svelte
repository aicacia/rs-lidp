<script lang="ts">
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { logout } from "$lib/common/state/auth.svelte";
	import { m } from "$lib/paraglide/messages";
	import type { PageProps } from "./$types";

	let { data }: PageProps = $props();

	const userInfo = $derived(data.userInfo);
	const displayName = $derived(
		userInfo?.name ||
			userInfo?.preferredUsername ||
			userInfo?.givenName ||
			userInfo?.email ||
			"User",
	);

	async function onSignOut() {
		await logout();
		goto(resolve("/signin"));
	}
</script>

<button class="btn danger" onclick={onSignOut}>{m.home_sign_out()}</button>
