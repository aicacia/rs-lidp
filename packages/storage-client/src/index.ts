export type StorageRequest =
    | { type: "addDevice"; deviceId: string }
    | { type: "removeDevice"; deviceId: string }
    | { type: "connectPeer"; peerId: string }
    | { type: "syncPeer"; peerId: string }
    | { type: "sendMessage"; peerId: string; payload: string | Uint8Array }
    | { type: "closeSession"; peerId: string }
    | { type: "readFile"; path: string }
    | { type: "writeFile"; path: string; content: string };

export type StorageResponse =
    | { ok: true; event?: StorageEvent; payload?: unknown }
    | { ok: false; error: string };

export type StorageEvent =
    | { type: "connected"; peerId: string }
    | { type: "messageReceived"; peerId: string; payload: string | Uint8Array }
    | { type: "closed"; peerId: string; reason?: string };

export type StorageClientOptions = {
    url: string;
};

// Internal protocol types
type BridgeRequest = StorageRequest & { requestId: number };

function getSocketConstructor(): typeof WebSocket | undefined {
    return globalThis.WebSocket;
}

let requestIdCounter = 0;

function getNextRequestId(): number {
    return ++requestIdCounter;
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
    private socket: WebSocket | null = null;
    private requestWaiters = new Map<
        number,
        (response: StorageResponse) => void
    >();
    private eventListeners = new Set<(event: StorageEvent) => void>();
    private socketPromise: Promise<WebSocket> | null = null;

    constructor(private readonly options: StorageClientOptions) {}

    static create(options: StorageClientOptions): StorageClient {
        return new StorageClient(options);
    }

    private async ensureSocket(): Promise<WebSocket> {
        if (this.socket) {
            if (this.socket.readyState === WebSocket.OPEN) {
                return this.socket;
            }
            this.socket = null;
            this.socketPromise = null;
        }

        if (this.socketPromise) {
            return this.socketPromise;
        }

        this.socketPromise = this.openSocket();
        return this.socketPromise;
    }

    private openSocket(): Promise<WebSocket> {
        return new Promise((resolve, reject) => {
            const WebSocketImpl = getSocketConstructor();

            if (!WebSocketImpl) {
                reject(
                    new Error("WebSocket is not available in this environment"),
                );
                return;
            }

            const socket = new WebSocketImpl(this.options.url);

            const onOpen = () => {
                cleanup();
                this.socket = socket;
                resolve(socket);
            };

            const onError = () => {
                cleanup();
                this.socket = null;
                this.socketPromise = null;
                reject(
                    new Error(
                        `Failed to connect to storage bridge at ${this.options.url}`,
                    ),
                );
            };

            const onMessage = (event: MessageEvent) => {
                this.handleMessage(String(event.data));
            };

            const cleanup = () => {
                socket.removeEventListener("open", onOpen);
                socket.removeEventListener("error", onError);
                socket.removeEventListener("message", onMessage);
            };

            socket.addEventListener("open", onOpen);
            socket.addEventListener("error", onError);
            socket.addEventListener("message", onMessage);
        });
    }

    private handleMessage(data: string): void {
        try {
            const parsed = JSON.parse(data);

            // Check if this is a response (has requestId and ok/error properties)
            if (
                "requestId" in parsed &&
                ("ok" in parsed || "error" in parsed)
            ) {
                const requestId = parsed.requestId;
                const waiter = this.requestWaiters.get(requestId);
                if (waiter) {
                    this.requestWaiters.delete(requestId);
                    waiter(parsed as StorageResponse);
                }
            } else if ("type" in parsed) {
                // This is an event
                const event = parsed as StorageEvent;
                for (const listener of this.eventListeners) {
                    listener(event);
                }
            }
        } catch {
            // Ignore malformed messages
        }
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
        const socket = await this.ensureSocket();
        const requestId = getNextRequestId();

        const message: BridgeRequest = {
            ...request,
            requestId,
        };

        return new Promise<TResponse>((resolve, reject) => {
            const timeout = setTimeout(() => {
                this.requestWaiters.delete(requestId);
                reject(
                    new Error(`storage request timeout for ${request.type}`),
                );
            }, 30000); // 30 second timeout

            this.requestWaiters.set(requestId, (response: StorageResponse) => {
                clearTimeout(timeout);
                if (response.ok) {
                    resolve(response as TResponse);
                } else {
                    reject(new Error(response.error));
                }
            });

            try {
                socket.send(JSON.stringify(message));
            } catch (error) {
                clearTimeout(timeout);
                this.requestWaiters.delete(requestId);
                reject(
                    error instanceof Error ? error : new Error(String(error)),
                );
            }
        });
    }

    listen(): AsyncIterable<StorageEvent> {
        const eventQueue: StorageEvent[] = [];
        const waiters: Array<() => void> = [];

        const listener = (event: StorageEvent) => {
            eventQueue.push(event);
            const waiter = waiters.shift();
            if (waiter) {
                waiter();
            }
        };

        // Ensure socket is open before adding listener
        this.ensureSocket().catch((error) => {
            console.error("Failed to open socket for listening", error);
        });

        this.eventListeners.add(listener);

        return {
            async *[Symbol.asyncIterator]() {
                while (true) {
                    if (eventQueue.length > 0) {
                        yield eventQueue.shift() as StorageEvent;
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

/**
 * Reads the contents of a file from the storage bridge.
 * @param client The StorageClient instance
 * @param path The relative path to the file
 * @returns The file contents as a string
 */
export async function readStorageFile(
    client: StorageClient,
    path: string,
): Promise<string> {
    const response = await client.request<StorageResponse>({
        type: "readFile",
        path,
    });

    if (!response.ok) {
        throw new Error(response.error);
    }

    if (typeof response.payload !== "string") {
        throw new Error("Expected string payload from readFile");
    }

    return response.payload;
}

/**
 * Writes content to a file in the storage bridge.
 * Creates the file if it doesn't exist, and creates parent directories as needed.
 * @param client The StorageClient instance
 * @param path The relative path to the file
 * @param content The content to write
 */
export async function writeStorageFile(
    client: StorageClient,
    path: string,
    content: string,
): Promise<void> {
    const response = await client.request<StorageResponse>({
        type: "writeFile",
        path,
        content,
    });

    if (!response.ok) {
        throw new Error(response.error);
    }
}
