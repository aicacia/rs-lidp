import type { TokenResponse, UserInfo } from "@aicacia/lidp-client";
import { OidcClientError } from "@aicacia/oidc-client";
import { createStorage } from "@aicacia/svelte-headless";
import { browser } from "$app/environment";
import { lidpApi, setAuthToken } from "./lidpClient.svelte";

const tokenResponseStorage = createStorage<TokenResponse | null>("token", null);

let tokenExpiryTimeout: ReturnType<typeof setTimeout> | null = null;
let tokenGeneration = 0;
let currentUserInfoFailureReason: CurrentUserInfoFailureReason | null = null;

type CurrentUserInfoFailureReason =
    | "NO_TOKEN_RESPONSE"
    | "NO_ACCESS_TOKEN"
    | "NO_USERINFO_ENDPOINT"
    | "HTTP_ERROR"
    | "JSON_PARSE_ERROR"
    | "NETWORK_TIMEOUT"
    | "NETWORK_ERROR"
    | "INVALID_USERINFO_RESPONSE"
    | "GRPC_UNAUTHENTICATED"
    | "GRPC_UNAVAILABLE"
    | "UNKNOWN";

export function getTokenResponse(): TokenResponse | null {
    return tokenResponseStorage.item;
}

export function getCurrentUserInfoFailureReason(): CurrentUserInfoFailureReason | null {
    return currentUserInfoFailureReason;
}

export function setTokenResponse(tokenResponse: TokenResponse | null): void {
    tokenGeneration += 1;
    const scheduledGeneration = tokenGeneration;

    if (!tokenResponse) {
        clearTokenExpiryTimeout();
        clearAuthState();
        return;
    }

    clearTokenExpiryTimeout();
    const expiresIn = tokenResponse.expiresIn;
    if (expiresIn !== undefined && expiresIn !== null && expiresIn > 0) {
        const expiresInMs = Math.max(0, expiresIn) * 1000;
        tokenExpiryTimeout = setTimeout(() => {
            void onTokenExpired(scheduledGeneration, tokenResponse);
        }, expiresInMs);
    }

    tokenResponseStorage.item = tokenResponse;
}

export async function getCurrentUserInfo(): Promise<UserInfo | null> {
    const tokenResponse = getTokenResponse();
    if (!tokenResponse) {
        currentUserInfoFailureReason = "NO_TOKEN_RESPONSE";
        console.log("No token response found");
        return null;
    }

    try {
        const userInfo = await lidpApi.userinfo();
        currentUserInfoFailureReason = null;
        return userInfo;
    } catch (e) {
        currentUserInfoFailureReason = toCurrentUserInfoFailureReason(e);
        console.error("getCurrentUserInfo error", {
            reason: currentUserInfoFailureReason,
            error: e,
        });
        return null;
    }
}

export async function logout(): Promise<void> {
    setTokenResponse(null);
}

if (browser) {
    $effect.root(() => {
        $effect(() => {
            const tokenResponse = getTokenResponse();
            setAuthToken(tokenResponse?.accessToken ?? null);

            return () => {
                clearTokenExpiryTimeout();
            };
        });
    });
}

function clearTokenExpiryTimeout(): void {
    if (tokenExpiryTimeout !== null) {
        clearTimeout(tokenExpiryTimeout);
        tokenExpiryTimeout = null;
    }
}

function clearAuthState(): void {
    tokenResponseStorage.item = null;
    currentUserInfoFailureReason = null;
}

function toCurrentUserInfoFailureReason(
    error: unknown,
): CurrentUserInfoFailureReason {
    if (error instanceof OidcClientError) {
        return error.code;
    }

    if (typeof error === "object" && error !== null && "code" in error) {
        const code = (error as { code?: unknown }).code;
        if (code === 16 || code === "UNAUTHENTICATED") {
            return "GRPC_UNAUTHENTICATED";
        }
        if (code === 14 || code === "UNAVAILABLE") {
            return "GRPC_UNAVAILABLE";
        }
    }

    return "UNKNOWN";
}

async function refreshTokenResponse(
    tokenResponse: TokenResponse,
): Promise<TokenResponse | null> {
    if (!tokenResponse.refreshToken) {
        return null;
    }

    try {
        const refreshed = await lidpApi.token({
            refreshToken: tokenResponse.refreshToken,
            scope: tokenResponse.scope,
        });

        return {
            ...refreshed,
            refreshToken: refreshed.refreshToken ?? tokenResponse.refreshToken,
            scope: refreshed.scope ?? tokenResponse.scope,
        };
    } catch (e) {
        console.error("refresh token error", e);
        return null;
    }
}

async function onTokenExpired(
    scheduledGeneration: number,
    tokenResponse: TokenResponse,
): Promise<void> {
    if (tokenGeneration !== scheduledGeneration) {
        return;
    }

    const refreshed = await refreshTokenResponse(tokenResponse);
    if (tokenGeneration !== scheduledGeneration) {
        return;
    }

    if (refreshed) {
        setTokenResponse(refreshed);
        return;
    }

    setTokenResponse(null);
}
