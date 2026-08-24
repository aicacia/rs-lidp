import { StorageClient } from "@aicacia/storage-client";
import { invoke } from "@tauri-apps/api/core";

function normalizeStorageBridgeUrl(url: string | null): string | null {
    return url ? url.trim().replace(/\/$/, "") : null;
}

async function fetchStorageBridgeUrl(): Promise<string | null> {
    const bridgeUrl = await invoke<string>("get_storage_bridge_url");
    return normalizeStorageBridgeUrl(bridgeUrl);
}

export type StorageBridgeConfig = {
    storageBridgeUrl: string;
};

export async function getStorageBridgeConfig(): Promise<StorageBridgeConfig | null> {
    const storageBridgeUrl = await fetchStorageBridgeUrl();
    return storageBridgeUrl ? { storageBridgeUrl } : null;
}

export async function getStorageBridgeUrl(): Promise<string | null> {
    const storageBridgeConfig = await getStorageBridgeConfig();
    return storageBridgeConfig?.storageBridgeUrl ?? null;
}

export async function getStorageClient(): Promise<StorageClient | null> {
    const storageBridgeUrl = await getStorageBridgeUrl();
    return storageBridgeUrl
        ? StorageClient.create({ url: storageBridgeUrl })
        : null;
}

export async function openStorageBridgeTrustPage(): Promise<void> {
    await invoke("open_storage_bridge_trust_page");
}

export function isStorageBridgeNative(url: string): boolean {
    const normalized = normalizeStorageBridgeUrl(url);
    return normalized
        ? /^wss:\/\/(127\.0\.0\.1|localhost|storage\.localhost)(:\d+)?$/i.test(
              normalized,
          )
        : false;
}

export async function validateStorageBridgeUrl(
    baseUrl: string,
): Promise<boolean> {
    const normalizedBaseUrl = normalizeStorageBridgeUrl(baseUrl);

    if (!normalizedBaseUrl) {
        return false;
    }

    let parsedUrl: URL;

    try {
        parsedUrl = new URL(normalizedBaseUrl);
    } catch {
        return false;
    }

    const host = parsedUrl.hostname.toLowerCase();
    const allowedHosts = ["127.0.0.1", "localhost", "storage.localhost"];

    if (parsedUrl.protocol !== "wss:") {
        return false;
    }

    if (!allowedHosts.includes(host)) {
        return false;
    }

    const WebSocketImpl = globalThis.WebSocket;

    if (!WebSocketImpl) {
        return false;
    }

    return await new Promise<boolean>((resolve) => {
        const socket = new WebSocketImpl(normalizedBaseUrl);
        const timeout = setTimeout(() => {
            cleanup();
            resolve(false);
        }, 1500);

        const cleanup = () => {
            clearTimeout(timeout);
            socket.removeEventListener("open", onOpen);
            socket.removeEventListener("error", onError);
            socket.close();
        };

        const onOpen = () => {
            cleanup();
            resolve(true);
        };

        const onError = () => {
            cleanup();
            resolve(false);
        };

        socket.addEventListener("open", onOpen);
        socket.addEventListener("error", onError);
    });
}
