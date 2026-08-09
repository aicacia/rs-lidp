<script lang="ts">
	import { resolve } from "$app/paths";
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
</script>

<div class="flex flex-col gap-4">
	<h1 class="mb-0 text-4xl">Dashboard</h1>
	<div class="card secondary">
		<p class="mb-0">Signed in as <strong>{displayName}</strong></p>
	</div>

	<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
		<a class="card secondary default hover:opacity-90" href={resolve("/users")}>
			<h2 class="mb-1 text-2xl">Users</h2>
			<p class="mb-0 text-sm opacity-80">Browse and manage user records.</p>
		</a>
		<a
			class="card secondary default hover:opacity-90"
			href={resolve("/clients")}
		>
			<h2 class="mb-1 text-2xl">Clients</h2>
			<p class="mb-0 text-sm opacity-80">
				Browse and manage OAuth client registrations.
			</p>
		</a>
		<a class="card secondary default hover:opacity-90" href={resolve("/roles")}>
			<h2 class="mb-1 text-2xl">Roles</h2>
			<p class="mb-0 text-sm opacity-80">
				Create, review, and remove assignable roles.
			</p>
		</a>
	</div>
</div>
