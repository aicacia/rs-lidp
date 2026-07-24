<script lang="ts">
    import { getOidcClient } from "$lib/common/state/oidc.svelte";
    import { m } from "$lib/paraglide/messages";

    async function onSubmit(event: Event) {
        event.preventDefault();

        await getOidcClient().signin();
    }
</script>

<form onsubmit={onSubmit} class="flex flex-col">
	<input class="btn primary mt-4" type="submit" value={m.sign_in()} />
</form>
