import { handleNativeCallbackRequestUrl } from "@aicacia/native-fetch";
import { env } from "$env/dynamic/public";

import { getStorageBridgeConfig } from "../state/storageClient.svelte";
import { redirectToUrl } from "./redirectToUrl";

export async function handleDeepLink(urlStrings: string[]): Promise<void> {
    const [urlString] = urlStrings;
    if (!urlString) {
        return;
    }

    const url = new URL(urlString);

    const callbackUrl = await handleNativeCallbackRequestUrl(url, async (request) => {
        if (url.pathname === "/bridge-url") {
            const bridgeUrlConfig = await getStorageBridgeConfig();
            return new Response(
                JSON.stringify({
                    bridgeUrl: bridgeUrlConfig?.storageBridgeUrl ?? "",
                }),
                {
                    headers: {
                        "content-type": "application/json;charset=UTF-8",
                    },
                },
            );
        }

        const apiUrl = new URL(url.pathname + url.search, env.PUBLIC_LIDP_BASE_URL);
        return fetch(apiUrl, {
            method: request.method,
            headers: request.headers,
            body: request.method === "GET" || request.method === "HEAD"
                ? undefined
                : await request.arrayBuffer(),
        });
    });

    await redirectToUrl(callbackUrl);
}
