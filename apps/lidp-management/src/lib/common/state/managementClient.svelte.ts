import {
    Configuration,
    type ConfigurationParameters,
    DefaultApi,
} from "@aicacia/lidp-management-client";
import { createStorage } from "@aicacia/svelte-headless";
import { goto } from "$app/navigation";
import { resolve } from "$app/paths";
import { page } from "$app/state";
import { env } from "$env/dynamic/public";
import { setAfterSigninRedirectPathFromURL } from "./afterSignInRedirectPath.svelte";

const managementApiUrl = createStorage<URL | null>("management-api-url", null, {
    serializer: {
        parse(text: string): URL | null {
            return new URL(text);
        },
        stringify(object: URL | null): string {
            return urlToString(object);
        },
    },
});

let authToken = $state<string | undefined>();

const defaultManagementApiUrl = env.PUBLIC_LIDP_MANAGEMENT_BASE_URL
    ? new URL(env.PUBLIC_LIDP_MANAGEMENT_BASE_URL)
    : null;

if (managementApiUrl.item == null && defaultManagementApiUrl != null) {
    managementApiUrl.item = defaultManagementApiUrl;
}

const defaultConfigurationParameters: ConfigurationParameters = {
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
        return urlToBasePathString(managementApiUrl.item);
    },
    get fetchApi() {
        return fetch;
    },
    credentials: "same-origin",
};

export const managementConfiguration = new Configuration(
    defaultConfigurationParameters,
);

export const managementApi = new DefaultApi(managementConfiguration);

export function setManagementApiUrl(newManagementApiUrl: URL) {
    managementApiUrl.item = newManagementApiUrl;
}

export function getManagementApiUrl(): URL | null {
    return managementApiUrl.item;
}

export function getManagementApiBaseUrl(): string {
    return urlToBasePathString(managementApiUrl.item);
}

export async function validateManagementApiUrl(
    url: URL | null,
): Promise<boolean> {
    if (!url) {
        return false;
    }
    const configuration = new Configuration({
        ...defaultConfigurationParameters,
        basePath: urlToBasePathString(url),
    });
    const api = new DefaultApi(configuration);

    try {
        return (await api.version()) != null;
    } catch {
        return false;
    }
}

export function setAuthToken(newAuthToken?: string | null) {
    authToken = newAuthToken ?? undefined;
}

export function getAuthToken() {
    return authToken;
}

function urlToBasePathString(url: URL | null): string {
    return (
        url
            ?.toString()
            .replace(/\/openapi\.json\/?$/, "")
            .replace(/\/$/, "") ?? ""
    );
}

function urlToString(url: URL | null): string {
    return url?.toString().replace(/\/$/, "") ?? "";
}
