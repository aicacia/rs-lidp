import { handleNativeCallbackRequestUrl } from "@aicacia/native-fetch";

import { getStorageBridgeConfig } from "../state/storageClient.svelte";
import { redirectToUrl } from "./redirectToUrl";

export async function handleDeepLink(urlStrings: string[]): Promise<void> {
    const [urlString] = urlStrings;
    if (!urlString) {
        return;
    }

    const url = new URL(urlString);

    if (url.pathname !== "/bridge-url") {
        console.warn(`Unknown deep link: ${urlString}`);
        return;
    }

    const bridgeUrlConfig = await getStorageBridgeConfig();
    const callbackUrl = await handleNativeCallbackRequestUrl(
        url,
        () =>
            new Response(
                JSON.stringify({
                    bridgeUrl: bridgeUrlConfig?.storageBridgeUrl ?? "",
                }),
                {
                    headers: {
                        "content-type": "application/json;charset=UTF-8",
                    },
                },
            ),
    );

    await redirectToUrl(callbackUrl);
}
