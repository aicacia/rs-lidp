# Storage Bridge Authentication and Capability Plan

## Goal

The storage bridge must not expose the complete file store to every client that can open its WebSocket. A user or service account should be able to request only the folders and connection capabilities granted to its JWT subject.

The design must support:

- authenticated request/response access to JSON, text, and arbitrary data;
- streaming video and audio from files;
- WebSocket initially, with WebRTC and local transports later;
- the same policy model for users, groups, roles, and service accounts;
- separate HTTP API deployments while keeping the desktop app local and unified.

## Current implementation (verified)

The current bridge has these properties:

- It binds to `127.0.0.1` on an ephemeral port and publishes `wss://storage.localhost:<port>`.
- TLS uses a locally generated server certificate and installed CA. The server config uses `with_no_client_auth`; TLS client certificates do not authenticate callers.
- The WebSocket endpoint is `/`. `/trust` only serves the certificate trust page.
- The client sends `authorization` inside each JSON request. The browser-facing `StorageClient` currently has no token option or handshake-auth support.
- `validate_session` requires `Bearer <token>`, checks the known host, calls `lidp_service::oauth2::decode_jwt`, checks `iss`, `nbf`, `exp`, and non-empty `sub`, then creates `StorageService::for_session(&sub)`. `decode_jwt` only decodes claims; it explicitly does **not** verify the JWT signature. The `aud` claim is deserialized but not checked.
- `StorageBridge::new` automatically remembers `storage.localhost`, so the known-host check currently does not provide a meaningful user approval boundary for this local bridge.
- The bridge supports peer/device operations plus unrestricted `readFile`, `writeFile`, `listDir`, `createDir`, `deletePath`, `renamePath`, and `existsPath` requests.
- The server validates each request independently, but there is no authenticated connection/session state and no per-request folder, operation, transport, or content policy.
- `StorageService` rejects empty, absolute, and `..` paths. It joins paths under the session root and checks lexical `starts_with`, but it does not resolve or reject symlinks/junctions that can escape the root.
- File reads are converted to UTF-8 strings by the bridge. There is no binary data, range, media, or streaming protocol.
- Only WebSocket transport exists. WebRTC, Unix sockets, TCP, and QUIC are not implemented.

Therefore any syntactically valid, unexpired, not-yet-valid-correct, issuer-matching unsigned JWT with a chosen `sub` can access all operations under that subject's derived directory. Signature verification and audience validation are immediate blockers before treating this as authentication. The bridge is suitable only for the current trusted local-app boundary, not for an untrusted browser, remote client, or general service-account deployment.

## Design principles

1. **Authenticate before upgrade and again at session establishment.** TLS and a trusted host identify the endpoint; they do not authorize a storage operation.
2. **Authorize every request.** Never authorize only when the WebSocket is opened. A token may expire or a policy may change while the socket remains open.
3. **Use canonical, root-confined paths.** Resolve paths without allowing `..`, absolute paths, symlink escapes, or alternate path representations to bypass a grant.
4. **Default deny.** Missing scopes, folder grants, transport grants, or content grants must fail closed.
5. **JWT `sub` identifies the principal, not the complete policy.** Resolve grants from the identity/management service or a locally cached policy. Do not infer permissions from a subject string.
6. **Capability-limit the connection.** The client asks for a narrow capability; the server returns the intersection of requested and granted capabilities.
7. **Separate control-plane and data-plane access.** File metadata and WebRTC signaling are control operations. File bytes and media tracks are data operations and need independent authorization.

## Proposed protocol

### 1. WebSocket handshake

The client sends the bearer token using the WebSocket handshake header when possible:

```http
Authorization: Bearer <jwt>
```

For environments that cannot set headers, use a short-lived, single-use connection ticket obtained from the HTTP API. Do not put long-lived JWTs in URLs, browser history, or logs.

Before accepting the connection, the bridge validates:

- signature and key identity;
- `iss`, `aud`, `exp`, `nbf`, and optional `jti`;
- principal status and policy version;
- optional device/client binding (`cnf`, client ID, or equivalent);
- requested protocol version.

The bridge should return a session identifier and policy revision. The token is not a permanent permission grant: each operation should be checked against the current policy. This is not current behavior; today no bridge session is created and the token is revalidated independently on each request.

### 2. Capability request

Add an explicit connection request before file access:

```json
{
  "type": "openConnection",
  "path": "media/example.mp4",
  "access": "read",
  "transport": "websocket",
  "content": "video",
  "format": "bytes",
  "range": { "start": 0, "end": 1048576 }
}
```

Supported values should be enums, not arbitrary strings:

- `access`: `read`, `write`, `list`, `metadata`;
- `transport`: `websocket`, `webrtc`, `unix_socket`, `tcp`, `quic`;
- `content`: `text`, `json`, `data`, `audio`, `video`;
- `format`: `utf8`, `json`, `bytes`, `media_track`.

The response contains the granted intersection and connection details:

```json
{
  "type": "connectionOpened",
  "connectionId": "c_123",
  "transport": "websocket",
  "content": "video",
  "format": "bytes",
  "expiresAt": 1770000000,
  "maxBytes": 1048576
}
```

If the requested transport or content type is not granted, reject the request. Do not silently downgrade a security-sensitive request. A non-security-sensitive client may explicitly provide an ordered fallback list in a future protocol version.

### 3. Data requests

Bind every data frame or follow-up request to `connectionId`. The server stores the authorized canonical path, access mode, content type, byte/rate limits, and expiry with that connection. The client must not be able to change the path by sending a later `readFile` message.

For JSON and text, validate the declared format and enforce size limits. For binary data, use bounded chunks and support ranges. Never deserialize arbitrary client-provided types into privileged server commands.

## Authorization model

A policy for a principal is composed of folder grants and capability grants:

```json
{
  "principal": "user-or-service-id",
  "policyVersion": 42,
  "folders": [
    {
      "root": "projects/acme",
      "access": ["read", "list"],
      "transports": ["websocket", "webrtc"],
      "content": ["json", "text", "video", "audio"],
      "maxBytes": 1073741824,
      "expiresAt": 1770000000
    }
  ]
}
```

Group and role grants should be resolved by the identity/management API and combined using least privilege. A recommended rule is:

- folder access is the intersection of the requested path and at least one active grant;
- operation, transport, and content must each be allowed by that grant;
- an explicit deny overrides an allow;
- service accounts require a separate client/application grant and should not inherit interactive-user defaults accidentally.

The JWT may carry a short-lived policy reference or immutable coarse scopes, but folder grants should normally be fetched from a trusted policy store. Cache them briefly and invalidate by policy revision or subject revocation. Do not accept folder grants supplied by the client.

## Path enforcement

Implement one authorization function used by every file operation and connection type:

```text
authorize(principal, operation, user_path, transport, content)
  -> canonical_path + effective_limits
```

It must:

1. reject empty, absolute, and traversal paths according to the platform;
2. normalize separators and Unicode handling consistently;
3. resolve the path relative to the storage root;
4. inspect existing components for symlink/junction escapes (the current `storage-service` lexical check is not sufficient);
5. verify the canonical path is within an authorized folder;
6. verify the operation, transport, and content grants;
7. return the canonical path and limits for the handler to use.

Do not authorize based on string prefix (`projects/acme` must not match `projects/acme-private`). Apply this function to reads, writes, listings, creates, deletes, renames, peer operations that expose files, and future media serving.

## WebRTC media connections

WebRTC should not grant a second path to storage. The bridge first authorizes a `content: video` or `content: audio` connection, then creates a short-lived signaling ticket bound to:

- principal and connection ID;
- canonical file path;
- media kind and codec/format constraints;
- read-only access;
- expiry, maximum duration, and optional bitrate;
- one peer/session identity.

The signaling channel may run over the authenticated WebSocket or the HTTP API. The media server must validate the ticket before sending bytes and must not accept a client-supplied file path. A combined audio/video request should be represented as two authorized tracks or an explicit `content: audiovisual` capability, not inferred from a video grant.

## Other transports

Transport support is an authorization dimension, not just a configuration switch:

- **WebSocket:** first implementation; authenticated handshake and bounded frames.
- **WebRTC:** short-lived, connection-bound media tickets; DTLS/SRTP remains responsible for transport encryption.
- **Unix socket:** local OS credential checks plus the same JWT/policy checks where cross-process identity matters. Restrict socket permissions and never expose it outside the intended user/session.
- **TCP/QUIC:** require explicit policy grants, TLS/mutual authentication as appropriate, and replay-resistant connection tickets.

A transport must not be enabled merely because it appears in a client request. The server advertises only transports it supports and the policy allows.

## Phased implementation plan

### Phase 0: Contain the existing bridge

- Keep requiring authentication for every request, but move credentials out of the request body where possible: add a `StorageClient` token option and send `Authorization` during the WebSocket handshake. Retain a short-lived single-use ticket fallback for browser limitations.
- Keep the loopback bind as a deliberate boundary; do not treat the self-signed CA or known-host SQLite row as caller authentication.
- Add strict origin/host checks where applicable.
- Apply connection, message, file-size, and rate limits.
- Stop logging raw WebSocket frames and serialized responses. Log principal, operation, canonical path, transport, content, and decision without logging tokens or file contents.
- Make all unrecognized request types fail closed.
- Fix the async bridge test to await `StorageBridge::new(...)` after the constructor became async.

### Phase 1: Folder and operation authorization

- Add a policy provider interface backed by the management/identity API, with a local cache for the desktop app.
- Replace `decode_jwt` with `verify_jwt` using the issuer's trusted public key, and validate the expected algorithm, issuer, audience, expiry, not-before, and optionally `jti`. `StorageSessionClaims` already contains `aud`; it is currently unused. Add client ID and policy version/reference as supported by the issuer.
- Replace direct handler access with `authorize(...)` for every path operation. Keep `StorageService` lexical validation as a lower-level defense, not the policy layer.
- Add safe canonicalization/no-follow handling for existing path components and carefully handle non-existent write targets.
- Add tests for traversal, symlink/junction escape, prefix confusion, rename across folders, expired grants, and group/service-account policies.

### Phase 2: Capability negotiation

- Add `openConnection` and `connectionId` to `@aicacia/storage-client`, `crates/storage-iroh`, and the Tauri bridge protocol. There is currently no shared connection-opening protocol; `BridgeMessage::Request` wraps the existing `StorageRequest` enum.
- Implement `websocket` plus `text`, `json`, and bounded `data` connections. First change file reads to return bytes or chunked data rather than forcing UTF-8.
- Return effective limits and expiry; re-check policy on long-lived connections.
- Deprecate unrestricted `readFile`/`writeFile` messages or route them through the same authorization function. Existing peer operations must also be classified explicitly because they can expose or move data outside ordinary file requests.

### Phase 3: WebRTC media

- Add media metadata probing without exposing file contents.
- Authorize audio/video tracks independently.
- Implement short-lived signaling tickets bound to a canonical path and connection ID.
- Add duration, bitrate, concurrent-stream, and codec limits.
- Test that signaling cannot switch files or escalate from audio to video.

### Phase 4: Additional transports

- Add Unix socket support for local trusted processes with OS-level permissions.
- Add TCP/QUIC only when a deployment has a clear need and TLS/authentication design.
- Advertise transport capabilities and version the protocol.

## Acceptance criteria

- A valid JWT with no folder grant cannot read, list, write, or stream any file.
- A grant for `a/b` cannot access `a/b-private`, `a/b/../secret`, or a symlink outside the grant.
- A read-only JSON grant cannot write, request video, or select WebRTC.
- A video grant cannot be changed to another path after a WebRTC ticket is issued.
- Expired, revoked, or policy-version-mismatched sessions fail closed.
- Service accounts and users follow the same authorization path, with auditable principal and client identity.
- Tokens, connection tickets, and file contents never appear in logs.
- Existing WebSocket clients are migrated to explicit authentication and capability requests before unrestricted request variants are removed.

## Open decisions

- Where the desktop app obtains and caches folder policy when the identity API is offline.
- Whether policies are JWT claims, a management API response, or a signed policy document.
- Exact media formats/codecs and whether transcoding is in scope.
- Whether local Unix-socket clients need JWTs in addition to OS credentials.
- Revocation latency and the maximum permitted lifetime of connection tickets.
