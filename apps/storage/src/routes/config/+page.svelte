<script lang="ts" module>
    import * as v from "valibot";
    import { m } from "$lib/paraglide/messages";

    const configSchema = v.objectAsync({
        lidpApiUrl: v.pipeAsync(
            v.string(),
            v.nonEmpty(m.errors_message_required()),
            v.url(m.errors_message_invalid_url()),
            v.checkAsync(validateLidpApiUrl, m.errors_message_invalid_url()),
        ),
    });
</script>

<script lang="ts">
    import { createForm } from "@aicacia/svelte-forms";
    import { goto } from "$app/navigation";
    import { resolve } from "$app/paths";
    import Issues from "$lib/common/components/Issues.svelte";
    import {
        getLidpApiUrl,
        setLidpApiUrl,
        validateLidpApiUrl,
    } from "$lib/common/state/lidpClient.svelte";
    import { notifications } from "$lib/common/state/notifications.svelte";
    import { getStorageBridgeUrl } from "$lib/common/state/storageClient.svelte";

    const form = createForm(configSchema, {
        lidpApiUrl: getLidpApiUrl() ?? "",
    });

    $effect(() => {
        if (form.state === "valid") {
            console.log(form);
        }
    });

    async function showCertificateTrustNotice(): Promise<void> {
        const url = await getStorageBridgeUrl();
        if (url && /^wss:\/\//i.test(url)) {
            window.alert(
                "The storage bridge uses a self-signed certificate. Open the cert file and trust it before connecting, or the browser will block the WebSocket.",
            );
        }
    }

    async function onSubmit(event: SubmitEvent) {
        event.preventDefault();

        const [_input, output, error] = await form.validate();
        if (error) {
            console.error(error);
            notifications.add(m.errors_message_invalid_form(), "error");
            return;
        }

        setLidpApiUrl(output.lidpApiUrl);

        await goto(resolve("/"));
    }
</script>

<div class="flex grow flex-col items-center justify-center">
    <div class="card w-sm">
        <h1>{m.env_config()}</h1>
        <form onsubmit={onSubmit} class="flex flex-col gap-4">
            <label class="flex flex-col">
                {m.env_config_lipd_url()}
                <input
                    type="text"
                    aria-label={m.env_config_lipd_url()}
                    placeholder={m.env_config_lipd_url_placeholder()}
                    bind:value={form.fields.lidpApiUrl.value}
                />
                <Issues issues={form.fields.lidpApiUrl.issues} />
            </label>
            <button
                type="button"
                class="btn secondary"
                onclick={showCertificateTrustNotice}
            >
                Trust certificate
            </button>
            <input
                type="submit"
                value={m.env_config_save()}
                disabled={form.state !== "valid"}
                class="btn primary"
            />
        </form>
    </div>
</div>
