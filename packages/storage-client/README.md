# @aicacia/storage-client

A browser-facing client for the native storage app. This package does not speak Iroh directly. It sends app-level requests to the native storage app over WebSockets and receives sync events from the storage runtime.

## Responsibilities

- app-level device and peer operations
- native storage orchestration over WebSockets
- event subscription for sync and peer messages
- request/response bridge to the local native app

## Not responsibilities

- direct Iroh endpoint creation
- gossip relay configuration in the browser
- application cryptography or peer trust logic

```ts
import { StorageClient } from "@aicacia/storage-client";

const client = StorageClient.create({ url: "ws://localhost:3042" });
const peer = await client.request<{ peerId: string }>({
  type: "connectPeer",
  peerId: "device-123",
});

for await (const event of client.listen()) {
  if (event.type === "messageReceived") {
    console.log("sync event", event.peerId, event.payload);
  }
}
```
