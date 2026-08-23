import { generateState } from "./generateState.js";
import { openUrl } from "./openUrl.js";

export type NativeFetchInit = RequestInit & {
    callbackUrl?: string;
    channelName?: string;
    timeout?: number;
};

export type HandleNativeFetchCallbackOptions = {
    channelName?: string;
};

export const NATIVE_FETCH_CHANNEL_NAME = "native-fetch";
export const NATIVE_FETCH_RESPONSE_EVENT = "native-fetch-response";

export type NativeRequestJSON = {
    url: string;
    headers: Record<string, string>;
    method: string;
    body: string | null;
    state: string;
    callbackUrl: string;
};

export type NativeResponseJSON = {
    headers: Record<string, string>;
    status: number;
    statusText: string;
    body: string | null;
    state: string;
};

/** Alias for {@link NativeRequestJSON}. */
export type NativeRequest = NativeRequestJSON;

/** Alias for {@link NativeResponseJSON}. */
export type NativeResponse = NativeResponseJSON;

async function bodyInitToString(
    body: BodyInit | null | undefined,
): Promise<string | null> {
    if (body == null) {
        return null;
    }
    if (typeof body === "string") {
        return body;
    }
    return new Response(body).text();
}

function headersToRecord(headers: HeadersInit | undefined): Record<string, string> {
    if (!headers) {
        return {};
    }
    return Object.fromEntries(new Headers(headers));
}

export function nativeFetch(
    input: URL | RequestInfo,
    init?: NativeFetchInit,
): Promise<Response>;
export function nativeFetch(
    input: string | URL | Request,
    init?: NativeFetchInit,
): Promise<Response>;

/**
 * Opens a native protocol URL and waits for the native app to respond
 * by opening a callback URL with the response data.
 */
export async function nativeFetch(
    input: string | URL | RequestInfo | Request,
    init?: NativeFetchInit,
) {
    const originUrl = window.location.origin;
    const url = new URL(
        input instanceof Request ? input.url : input.toString(),
    );
    const state = generateState();
    const callbackUrl = init?.callbackUrl ?? `${originUrl}/native-callback`;
    const timeout = init?.timeout;
    const channelName = init?.channelName ?? NATIVE_FETCH_CHANNEL_NAME;
    const body = await bodyInitToString(init?.body);

    const native: NativeRequestJSON = {
        url: url.href,
        headers: headersToRecord(init?.headers),
        method: init?.method ?? "GET",
        body,
        state,
        callbackUrl,
    };
    url.searchParams.set("native", JSON.stringify(native));

    return new Promise<Response>((resolve, reject) => {
        let popupWindow: Window | null = null;
        let timeoutId: ReturnType<typeof setTimeout> | null = null;
        let responseChannel: BroadcastChannel | null = null;
        let channelListener: ((event: MessageEvent) => void) | null = null;

        function cleanup() {
            if (timeoutId) {
                clearTimeout(timeoutId);
            }
            if (responseChannel && channelListener) {
                responseChannel.removeEventListener("message", channelListener);
            }
            if (responseChannel) {
                responseChannel.close();
            }
            if (popupWindow && !popupWindow.closed) {
                popupWindow.close();
            }
        }

        if (timeout) {
            timeoutId = setTimeout(() => {
                cleanup();
                reject(new Error(`Native fetch timeout after ${timeout}ms`));
            }, timeout);
        }

        responseChannel = new BroadcastChannel(channelName);

        channelListener = (event: MessageEvent) => {
            if (event.data?.type !== NATIVE_FETCH_RESPONSE_EVENT) {
                return;
            }
            const nativeResponse = event.data.data as
                | NativeResponseJSON
                | undefined;
            if (nativeResponse?.state !== state) {
                return;
            }
            cleanup();
            resolve(
                new Response(nativeResponse.body, {
                    headers: nativeResponse.headers,
                    status: nativeResponse.status,
                    statusText: nativeResponse.statusText,
                }),
            );
        };

        responseChannel.addEventListener("message", channelListener);

        popupWindow = openUrl(url, {
            popup: true,
        });
    });
}

/**
 * Helper to handle native fetch callback in the callback page.
 * Call this in your callback route to send the response back to the waiting fetch.
 */
export function handleNativeFetchCallback(
    searchParams: URLSearchParams,
    {
        channelName = NATIVE_FETCH_CHANNEL_NAME,
    }: HandleNativeFetchCallbackOptions = {},
): void {
    const native = searchParams.get("native");
    if (!native) {
        console.warn("No native parameter in fetch callback");
        return;
    }
    const responseChannel = new BroadcastChannel(channelName);

    responseChannel.postMessage({
        type: NATIVE_FETCH_RESPONSE_EVENT,
        data: JSON.parse(native),
    });
    responseChannel.close();

    if (window.history.length > 1) {
        window.history.back();
    }
    window.close();
}

export async function handleNativeCallbackRequestUrl(
    requestUrlOrString: URL | string,
    callback: (request: Request) => Response | Promise<Response>,
): Promise<URL> {
    const requestUrl = new URL(requestUrlOrString);
    const nativeRequestParam = requestUrl.searchParams.get("native");
    if (!nativeRequestParam) {
        throw new Error("Missing `native` parameter");
    }
    const nativeRequest = JSON.parse(nativeRequestParam) as NativeRequestJSON;
    return handleNativeCallbackRequest(nativeRequest, callback);
}

export async function handleNativeCallbackRequest(
    nativeRequest: NativeRequestJSON,
    callback: (request: Request) => Response | Promise<Response>,
): Promise<URL> {
    const request = new Request(nativeRequest.url, {
        method: nativeRequest.method,
        headers: nativeRequest.headers,
        body: nativeRequest.body,
    });
    let nativeResponse: NativeResponseJSON;
    try {
        const response = await callback(request);
        nativeResponse = {
            headers: response.headers
                ? Object.fromEntries(response.headers)
                : {},
            status: response.status,
            statusText: response.statusText,
            body: await response.text(),
            state: nativeRequest.state,
        };
    } catch (error) {
        nativeResponse = {
            headers: {},
            status: 500,
            statusText: (error as Error).message,
            body: (error as Error).message,
            state: nativeRequest.state,
        };
    }

    const callbackUrl = new URL(nativeRequest.callbackUrl);
    callbackUrl.searchParams.set("native", JSON.stringify(nativeResponse));
    return callbackUrl;
}
