<script lang="ts">
	import { m } from "$lib/paraglide/messages";
	import LIdP from "./_LIdP.svelte";
</script>

<div class="flex grow flex-col items-center justify-center">
	<div class="card w-sm">
	    <h1>{m.lidp_config()}</h1>
		<LIdP />
	</div>
</div>
