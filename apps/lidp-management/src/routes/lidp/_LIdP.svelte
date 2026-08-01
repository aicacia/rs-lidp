<script lang="ts" module>
    import * as v from "valibot";
    import { m } from "$lib/paraglide/messages";

    const lIdpConfigSchema = v.object({
        url: v.pipe(
            v.string(),
            v.nonEmpty(m.errors_message_required()),
            v.url(m.errors_message_invalid_url()),
        ),
    });
</script>

<script lang="ts">
    import { createForm } from "@aicacia/svelte-forms";
    import { goto } from "$app/navigation";
    import { resolve } from "$app/paths";
    import Issues from "$lib/common/components/Issues.svelte";
    import {
        getManagementApiUrl,
        setManagementApiUrl,
        validateManagementApiUrl,
    } from "$lib/common/state/managementClient.svelte";

    const form = createForm(lIdpConfigSchema, {
        url: getManagementApiUrl()?.toString() ?? "",
    });

    async function onSubmit(e: SubmitEvent) {
        e.preventDefault();

        const [_input, output, error] = await form.validate();

        if (error) {
            return;
        }
        const lIdpApiUrl = new URL(output.url);
        if (!(await validateManagementApiUrl(lIdpApiUrl))) {
            form.fields.url.issues.push(m.errors_message_invalid_url());
            return;
        }
        setManagementApiUrl(lIdpApiUrl);

        await goto(resolve("/signin"));
    }
</script>

<form onsubmit={onSubmit} class="flex flex-col">
    <label class="flex flex-col">
        {m.lidp_config_url()}
        <input
            type="text"
            aria-label={m.lidp_config_url()}
            placeholder={m.lidp_config_url_placeholder()}
            bind:value={form.fields.url.value}
        />
        <Issues issues={form.fields.url.issues} />
    </label>
    <input class="btn primary mt-4" type="submit" value={m.lidp_save()} />
</form>
