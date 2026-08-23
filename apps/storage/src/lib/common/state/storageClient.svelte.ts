import {
    StorageClient,
    type StorageClientOptions,
} from "@aicacia/storage-client";
import { createStorage } from "@aicacia/svelte-headless";
import { isTauri } from "@tauri-apps/api/core";
import { env } from "$env/dynamic/public";

const defaultStorageBridgeUrl = "ws://127.0.0.1:3042";

export type StorageBridgeConfig = {
    storageBridgeUrl: string;
    requestUrl: string;
    eventUrl: string;
};

const storageBridgeUrl = createStorage<string>(
    "storage-bridge-url",
    (isTauri() ? defaultStorageBridgeUrl : env.PUBLIC_STORAGE_BRIDGE_URL) ??
        defaultStorageBridgeUrl,
);

function normalizeStorageBridgeUrl(url: string): string {
    return url.trim().replace(/\/$/, "");
}

function buildStorageClientOptions(baseUrl: string): StorageClientOptions {
    const normalizedBaseUrl = normalizeStorageBridgeUrl(baseUrl);

    return {
        url: normalizedBaseUrl,
        requestUrl: `${normalizedBaseUrl}/request`,
        eventUrl: `${normalizedBaseUrl}/events`,
    };
}

export function getStorageBridgeConfig(): StorageBridgeConfig {
    const baseUrl = normalizeStorageBridgeUrl(storageBridgeUrl.item);

    return {
        storageBridgeUrl: baseUrl,
        requestUrl: `${baseUrl}/request`,
        eventUrl: `${baseUrl}/events`,
    };
}

export function getStorageBridgeUrl(): string {
    return storageBridgeUrl.item;
}

export function setStorageBridgeUrl(newStorageBridgeUrl: string): void {
    storageBridgeUrl.item = normalizeStorageBridgeUrl(newStorageBridgeUrl);
}

export function createStorageBridgeClient(): StorageClient {
    return StorageClient.create(
        buildStorageClientOptions(storageBridgeUrl.item),
    );
}

let storageClient = $derived.by(() => createStorageBridgeClient());

export function getStorageClient(): StorageClient {
    return storageClient;
}

export function isStorageBridgeNative(): boolean {
    return storageBridgeUrl.item.startsWith(defaultStorageBridgeUrl);
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

    if (parsedUrl.protocol !== "ws:" && parsedUrl.protocol !== "wss:") {
        return false;
    }

    const WebSocketImpl = globalThis.WebSocket;

    if (!WebSocketImpl) {
        return false;
    }

    return await new Promise<boolean>((resolve) => {
        const socket = new WebSocketImpl(`${normalizedBaseUrl}/request`);
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
