<script lang="ts">
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
    import { getOidcClient } from "$lib/common/state/oidc.svelte";
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
	    await getOidcClient().signout();
		await goto(resolve("/signin"));
	}
</script>

<div class="flex grow flex-col items-center justify-center">
	<div class="card w-sm">
		<div class="flex flex-col items-center gap-4">
			<h2 class="text-2xl">{m.home_welcome({ displayName })}</h2>
			<p class="text-center text-gray-600 dark:text-gray-400">
				{m.home_signed_in_message()}
			</p>
			<button class="btn danger" onclick={onSignOut}>{m.home_sign_out()}</button
			>
		</div>
	</div>
</div>
