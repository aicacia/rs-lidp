import { describe, expect, it } from "vitest";

import { readStorageFile, StorageClient, writeStorageFile } from "./index";

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

    describe("file operations", () => {
        it("exposes readStorageFile helper function", () => {
            expect(typeof readStorageFile).toBe("function");
        });

        it("exposes writeStorageFile helper function", () => {
            expect(typeof writeStorageFile).toBe("function");
        });

        it("creates a read file request with correct type and path", async () => {
            const client = StorageClient.create({ url: "ws://localhost:3042" });
            let capturedRequest: unknown;

            const originalRequest = client.request.bind(client);
            client.request = async (request) => {
                capturedRequest = request;
                throw new Error("mock error");
            };

            try {
                await readStorageFile(client, "test/file.txt");
            } catch (e) {
                // Expected to throw due to our mock
            }

            expect(capturedRequest).toEqual({
                type: "readFile",
                path: "test/file.txt",
            });
        });

        it("creates a write file request with correct type, path and content", async () => {
            const client = StorageClient.create({ url: "ws://localhost:3042" });
            let capturedRequest: unknown;

            const originalRequest = client.request.bind(client);
            client.request = async (request) => {
                capturedRequest = request;
                throw new Error("mock error");
            };

            try {
                await writeStorageFile(
                    client,
                    "test/file.txt",
                    "Hello, World!",
                );
            } catch (e) {
                // Expected to throw due to our mock
            }

            expect(capturedRequest).toEqual({
                type: "writeFile",
                path: "test/file.txt",
                content: "Hello, World!",
            });
        });
    });
});
