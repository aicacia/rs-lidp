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

let lIdpApiUrl = createStorage<URL | null>("lidp-api-url", null, {
  serializer: {
    parse(text: string): URL | null {
      return new URL(text);
    },
    stringify(object: URL | null): string {
      return urlToString(object);
    },
  },
});
let isLIdpApiIsNative = $derived.by(() => lIdpApiUrl.item?.protocol === "lidp:")
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
    return isLIdpApiIsNative ? fetch : fetch
  },
  accessToken() {
    return authToken as string;
  },
  get basePath() {
    return urlToString(lIdpApiUrl.item);
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

export function setLIdpApiUrl(newLIdpApiUrl: URL) {
  lIdpApiUrl.item = newLIdpApiUrl;
}
export function getLIdpApiUrl(): URL | null {
  return lIdpApiUrl.item;
}

export async function validateLIdpApiUrl(url: URL | null): Promise<boolean> {
  if (!url) {
    return false;
  }
  const configuration = new Configuration({
    ...defaultConfigurationParameters,
    basePath: urlToString(url),
});
  const api = new DefaultApi(configuration);

  try {
    return (await api.version()) != null;
  } catch {
    return false;
  }
}

function urlToString(url: URL | null): string {
  return url?.toString().replace(/\/$/, "") ?? "";
}
