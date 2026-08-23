export type StorageRequest =
    | { type: "addDevice"; deviceId: string }
    | { type: "removeDevice"; deviceId: string }
    | { type: "connectPeer"; peerId: string }
    | { type: "syncPeer"; peerId: string }
    | { type: "sendMessage"; peerId: string; payload: string | Uint8Array }
    | { type: "closeSession"; peerId: string };

export type StorageResponse =
    | { ok: true; event?: StorageEvent; payload?: unknown }
    | { ok: false; error: string };

export type StorageEvent =
    | { type: "connected"; peerId: string }
    | { type: "messageReceived"; peerId: string; payload: string | Uint8Array }
    | { type: "closed"; peerId: string; reason?: string };

export type StorageClientOptions = {
    url: string;
    requestUrl?: string;
    eventUrl?: string;
};

function getSocketConstructor(): typeof WebSocket | undefined {
    return globalThis.WebSocket;
}

export class PeerSession {
    constructor(
        private readonly client: StorageClient,
        public readonly peerId: string,
    ) {}

    async send(payload: string | Uint8Array): Promise<void> {
        const response = await this.client.request<StorageResponse>({
            type: "sendMessage",
            peerId: this.peerId,
            payload,
        });

        if (!response.ok) {
            throw new Error(response.error);
        }
    }

    async close(): Promise<void> {
        const response = await this.client.request<StorageResponse>({
            type: "closeSession",
            peerId: this.peerId,
        });

        if (!response.ok) {
            throw new Error(response.error);
        }
    }

    async *events(): AsyncIterable<StorageEvent> {
        for await (const event of this.client.listen()) {
            if (
                event.type === "messageReceived" &&
                event.peerId !== this.peerId
            ) {
                continue;
            }
            if (
                event.type !== "messageReceived" &&
                event.peerId !== this.peerId
            ) {
                continue;
            }
            yield event;
        }
    }
}

export class StorageClient {
    constructor(private readonly options: StorageClientOptions) {}

    static create(options: StorageClientOptions): StorageClient {
        return new StorageClient(options);
    }

    async connectPeer(peerId: string): Promise<PeerSession> {
        const response = await this.request<StorageResponse>({
            type: "connectPeer",
            peerId,
        });

        if (!response.ok) {
            throw new Error(response.error);
        }

        return this.peerSession(peerId);
    }

    peerSession(peerId: string): PeerSession {
        if (!peerId || !peerId.trim()) {
            throw new Error("peerSession requires a peer id");
        }
        return new PeerSession(this, peerId.trim());
    }

    async request<TResponse = unknown>(
        request: StorageRequest,
    ): Promise<TResponse> {
        const WebSocketImpl = getSocketConstructor();

        if (!WebSocketImpl) {
            throw new Error("WebSocket is not available in this environment");
        }

        const targetUrl = this.options.requestUrl ?? this.options.url;
        const socket = new WebSocketImpl(targetUrl);

        return new Promise<TResponse>((resolve, reject) => {
            const onOpen = () => {
                socket.send(JSON.stringify(request));
            };

            const onMessage = (event: MessageEvent) => {
                try {
                    const payload = JSON.parse(String(event.data)) as TResponse;
                    cleanup();
                    resolve(payload);
                } catch (error) {
                    cleanup();
                    reject(
                        error instanceof Error
                            ? error
                            : new Error(String(error)),
                    );
                }
            };

            const onError = () => {
                cleanup();
                reject(new Error(`storage request failed for ${request.type}`));
            };

            const cleanup = () => {
                socket.removeEventListener("open", onOpen);
                socket.removeEventListener("message", onMessage);
                socket.removeEventListener("error", onError);
                socket.close();
            };

            socket.addEventListener("open", onOpen);
            socket.addEventListener("message", onMessage);
            socket.addEventListener("error", onError);
        });
    }

    listen(): AsyncIterable<StorageEvent> {
        const WebSocketImpl = getSocketConstructor();

        if (!WebSocketImpl) {
            throw new Error("WebSocket is not available in this environment");
        }

        const targetUrl = this.options.eventUrl ?? this.options.url;
        const socket = new WebSocketImpl(targetUrl);
        const queue: StorageEvent[] = [];
        const waiters: Array<() => void> = [];

        socket.addEventListener("message", (event: MessageEvent) => {
            try {
                const payload = JSON.parse(String(event.data)) as StorageEvent;
                queue.push(payload);
                const waiter = waiters.shift();
                if (waiter) {
                    waiter();
                }
            } catch {
                // ignore malformed event frames until the native bridge begins sending typed payloads
            }
        });

        return {
            async *[Symbol.asyncIterator]() {
                while (true) {
                    if (queue.length > 0) {
                        yield queue.shift() as StorageEvent;
                        continue;
                    }

                    await new Promise<void>((resolve) => {
                        waiters.push(resolve);
                    });
                }
            },
        };
    }
}
