import {
  Configuration,
  type ConfigurationParameters,
  DefaultApi,
} from "@aicacia/lidp-client";
import { goto } from "$app/navigation";
import { resolve } from "$app/paths";
import { page } from "$app/state";
import { setAfterSigninRedirectPathFromURL } from "./afterSignInRedirectPath.svelte";
import { createStorage } from "@aicacia/svelte-headless";
import { env } from "$env/dynamic/public";

const lidpApiUrl = createStorage<string | null>("lidp-api-url", env.PUBLIC_LIDP_BASE_URL ?? null);
let lidpApiIsNative = $derived.by(() => lidpApiUrl.item?.startsWith("lidp:"));
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
  get fetchApi() {
    // TODO: create a IPC fetch API for tauri
    return lidpApiIsNative ? fetch : fetch
  },
  accessToken() {
    return authToken as string;
  },
  get basePath() {
    return lidpApiUrl.item;
  },
  credentials: "same-origin",
};

export const lidpConfiguration = new Configuration(
  defaultConfigurationParameters,
);

export const lidpApi = new DefaultApi(lidpConfiguration);

export function setAuthToken(newAuthToken?: string | null) {
  authToken = newAuthToken ?? undefined;
}
export function getAuthToken() {
  return authToken;
}

export function setLidpApiUrl(newLidpApiUrl: string) {
  lidpApiUrl.item = newLidpApiUrl;
}
export function getLidpApiUrl(): string | null {
  return lidpApiUrl.item;
}

export function isLidpApiNative(): boolean {
  return lidpApiIsNative;
}

export async function validateLidpApiUrl(basePath: string): Promise<boolean> {
  if (!basePath) {
    return false;
  }
  const configuration = new Configuration({
    ...defaultConfigurationParameters,
    basePath,
});
  const api = new DefaultApi(configuration);

  try {
    return (await api.version()) != null;
  } catch {
    return false;
  }
}
