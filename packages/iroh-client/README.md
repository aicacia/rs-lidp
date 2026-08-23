# Aicacia Iroh Client

This package exposes a small browser-friendly wrapper around the generic iroh gossip channel. The core model is a topic-based channel, not a legacy chat-specific room object.

## Basic chat example

```ts
import { IrohClient } from "@aicacia/iroh-client";

const client = await IrohClient.create({ relayUrl: "https://relay.n0.iroh.link" });

const room = await client.createChannel();
console.log("channel id:", room.id());

for await (const event of room.events()) {
  if (event.type === "joined") {
    console.log("peers:", event.neighbors);
  }

  if (event.type === "messageReceived") {
    if (event.text) {
      console.log("text:", event.text);
    }
    if (event.binary) {
      console.log("binary:", new Uint8Array(event.binary));
    }
  }
}

await room.broadcast("hello from browser");

const ticket = room.ticket({ includeSelf: true, includeBootstrap: true });
const peerRoom = await client.joinChannel(ticket);
await peerRoom.send("neighbor-scoped message");
```

## Text and binary messages

`broadcast`, `broadcastNeighbor`, and `send` all accept either a JavaScript string or a `Uint8Array`.

```ts
await room.broadcast("to everyone in the channel");
await room.broadcast(new TextEncoder().encode("raw bytes to everyone"));
await room.broadcastNeighbor("to the current neighbor set only");
await room.send("peer-id-or-ticket", "direct peer message");
```

## Peer-scoped sessions

For one-to-one delivery, use a peer-scoped session instead of a broadcast channel. This keeps the transport focused on routing while leaving confidentiality, identity checks, and payload encryption to the application layer.

```ts
const client = await IrohClient.create({ relayUrl: "https://relay.n0.iroh.link" });
const channel = await client.createChannel();
const peerSession = channel.peerSession("peer-id-123");

for await (const event of peerSession.events()) {
  if (event.type === "messageReceived") {
    console.log("private message:", event.text ?? event.binary);
  }
}

await peerSession.send("encrypted or otherwise app-managed data");
```

`peerSession` filters incoming messages to the selected peer, but it does not provide encryption or identity verification. If privacy matters, the application must encrypt payloads and authenticate the peer before trusting the message.

## broadcast vs broadcastNeighbor vs send

- `broadcast(...)` sends to the full channel topic.
- `broadcastNeighbor(...)` sends only to the current direct-neighbor set.
- `send(peerOrTicket, payload)` requires a known peer id or ticket and is intended for direct peer delivery.
- `peerSession(peerId)` is the explicit one-to-one routing helper for app-level privacy and peer-specific handling.

The underlying transport is still gossip-based, so direct peer delivery is a layer above the raw gossip primitive. In practice, the common chat pattern is `broadcast(...)` for room messages and `peerSession(...)` or `send(...)` for direct peer communication.
