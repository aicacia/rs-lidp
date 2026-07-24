<script lang="ts" module>
	interface PageProps {
		data: PageData;
	}
</script>

<script lang="ts">
	import { m } from "$lib/paraglide/messages";
	import SignIn from "./_SignIn.svelte";
	import type { PageData } from "./$types";

	let { data }: PageProps = $props();
</script>

<div class="flex grow flex-col items-center justify-center">
	<div class="card w-sm">
		<h1>{m.sign_in()}</h1>
		<SignIn configuration={data.configuration} />
	</div>
</div>
