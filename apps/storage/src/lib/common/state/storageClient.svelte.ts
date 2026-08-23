import { StorageClient } from "@aicacia/storage-client";
import { createStorage } from "@aicacia/svelte-headless";

export type StorageBridgeConfig = {
    storageBridgeUrl: string;
};

const storageBridgeUrl = createStorage<string | null>(
    "storage-bridge-url",
    null,
);

function normalizeStorageBridgeUrl(url: string | null): string | null {
    return url ? url.trim().replace(/\/$/, "") : null;
}

export function getStorageBridgeConfig(): StorageBridgeConfig | null {
    const normalized = normalizeStorageBridgeUrl(storageBridgeUrl.item);
    return normalized ? { storageBridgeUrl: normalized } : null;
}

export function getStorageBridgeUrl(): string | null {
    return normalizeStorageBridgeUrl(storageBridgeUrl.item);
}

export function setStorageBridgeUrl(newStorageBridgeUrl: string): void {
    storageBridgeUrl.item = normalizeStorageBridgeUrl(newStorageBridgeUrl);
}

export function createStorageBridgeClient(): StorageClient | null {
    const url = normalizeStorageBridgeUrl(storageBridgeUrl.item);
    return url ? StorageClient.create({ url }) : null;
}

let storageClient = $derived.by(() => createStorageBridgeClient());

export function getStorageClient(): StorageClient | null {
    return storageClient;
}

export function isStorageBridgeNative(): boolean {
    const url = normalizeStorageBridgeUrl(storageBridgeUrl.item);
    return url
        ? /^wss:\/\/(127\.0\.0\.1|localhost|storage\.local)(:\d+)?$/i.test(url)
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
    const allowedHosts = ["127.0.0.1", "localhost", "storage.local"];

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
