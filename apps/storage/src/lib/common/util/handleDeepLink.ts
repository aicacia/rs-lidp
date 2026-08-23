import { handleNativeCallbackRequestUrl } from "@aicacia/oidc-client";

import { getStorageBridgeConfig } from "../state/storageClient.svelte";
import { redirectToUrl } from "./redirectToUrl";

export async function handleDeepLink(urlStrings: string[]): Promise<void> {
    console.log("handleDeepLink", urlStrings);

    const [urlString] = urlStrings;
    if (!urlString) {
        return;
    }

    const url = new URL(urlString);

    console.debug("Deep link received", url);

    switch (url.pathname) {
        case "/config": {
            const callbackUrl = await handleNativeCallbackRequestUrl(
                url,
                () =>
                    new Response(JSON.stringify(getStorageBridgeConfig()), {
                        headers: {
                            "content-type": "application/json;charset=UTF-8",
                        },
                    }),
            );

            await redirectToUrl(callbackUrl);
            break;
        }
        default: {
            console.warn(`Unknown deep link: ${urlString}`);
            break;
        }
    }
}
