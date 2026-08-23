import { describe, expect, it } from "vitest";

import { StorageClient } from "./index";

describe("storage client", () => {
    it("creates a client and exposes peer session operations", () => {
        const client = StorageClient.create({ url: "ws://localhost:3042" });

        expect(client).toBeDefined();
        expect(typeof client.connectPeer).toBe("function");
        expect(typeof client.request).toBe("function");
        expect(client.peerSession("peer-123").peerId).toBe("peer-123");
        expect(() => client.peerSession("")).toThrow(
            "peerSession requires a peer id",
        );
    });

    it("rejects requests when websocket is unavailable in this environment", async () => {
        const original = globalThis.WebSocket;
        Reflect.deleteProperty(globalThis, "WebSocket");

        try {
            const client = StorageClient.create({ url: "ws://localhost:3042" });
            await expect(
                client.request({ type: "connectPeer", peerId: "peer-123" }),
            ).rejects.toThrow("WebSocket is not available");
        } finally {
            if (original) {
                globalThis.WebSocket = original;
            }
        }
    });
});
