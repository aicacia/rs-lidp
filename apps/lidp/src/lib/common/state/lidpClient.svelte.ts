import {
    Configuration,
    type ConfigurationParameters,
    DefaultApi,
} from "@aicacia/lidp-client";
import { goto } from "$app/navigation";
import { resolve } from "$app/paths";
import { page } from "$app/state";
import { setAfterSigninRedirectPathFromURL } from "./afterSignInRedirectPath.svelte";

let idpApiUrl = $state<string>("https://lidp-api.localhost:1337");
let authToken = $state<string | undefined>();

export const defaultConfigurationParameters: ConfigurationParameters = {
    middleware: [
        {
            pre: async (context) => ({
                ...context,
                init: {
                    ...context.init,
                    mode: "cors",
                },
            }),
        },
        {
            post: async (context) => {
                if (context.response.status === 401) {
                    setAfterSigninRedirectPathFromURL(page.url);
                    authToken = undefined;
                    await goto(resolve("/signin"));
                }
                return context.response;
            },
        },
    ],
    accessToken() {
        return authToken as string;
    },
    get basePath() {
        return new URL(idpApiUrl).origin;
    },
    credentials: "same-origin",
};

export const lidpConfiguration = new Configuration(
    defaultConfigurationParameters,
);

export const lidpApi = new DefaultApi(lidpConfiguration);

export function setAuthToken(newAuthToken: string | null) {
    authToken = newAuthToken ?? undefined;
}
export function getAuthToken() {
    return authToken;
}
