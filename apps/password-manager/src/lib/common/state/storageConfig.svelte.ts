import { nativeFetch } from "@aicacia/oidc-client";
import { createStorage } from "@aicacia/svelte-headless";

export type StorageBridgeConfig = {
    storageBridgeUrl: string;
    requestUrl: string;
    eventUrl: string;
};

const storageBridgeConfig = createStorage<StorageBridgeConfig | null>(
    "storage-bridge-config",
    null,
);

export function getStorageBridgeConfig(): StorageBridgeConfig | null {
    return storageBridgeConfig.item;
}

export function getStorageBridgeUrl(): string | null {
    return storageBridgeConfig.item?.storageBridgeUrl ?? null;
}

export function getStorageBridgeRequestUrl(): string | null {
    return storageBridgeConfig.item?.requestUrl ?? null;
}

export function getStorageBridgeEventUrl(): string | null {
    return storageBridgeConfig.item?.eventUrl ?? null;
}

export async function loadStorageBridgeConfig(): Promise<StorageBridgeConfig | null> {
    try {
        const response = await nativeFetch("storage://app/config");
        const config = (await response.json()) as StorageBridgeConfig;

        storageBridgeConfig.item = config;

        return config;
    } catch (error) {
        console.warn("Failed to load storage bridge config", error);
        return storageBridgeConfig.item;
    }
}
