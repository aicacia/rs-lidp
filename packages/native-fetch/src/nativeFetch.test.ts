import { expect, test } from "vitest";
import { isNativeProtocol } from "./isNativeProtocol.js";
import {
    handleNativeCallbackRequest,
    handleNativeCallbackRequestUrl,
} from "./nativeFetch.js";

test("isNativeProtocol treats custom schemes as native", () => {
    expect(isNativeProtocol(new URL("storage://app/config"))).toBe(true);
    expect(isNativeProtocol(new URL("https://example.com"))).toBe(false);
});

test("handleNativeCallbackRequest builds callback URL with response payload", async () => {
    const callbackUrl = await handleNativeCallbackRequest(
        {
            url: "storage://app/config",
            headers: { accept: "application/json" },
            method: "GET",
            body: null,
            state: "abc123",
            callbackUrl: "https://app.example/native-callback",
        },
        () =>
            new Response(JSON.stringify({ ok: true }), {
                headers: { "content-type": "application/json" },
            }),
    );

    expect(callbackUrl.origin).toBe("https://app.example");
    expect(callbackUrl.pathname).toBe("/native-callback");

    const native = callbackUrl.searchParams.get("native");
    expect(native).toBeTruthy();
    if (!native) {
        return;
    }

    const response = JSON.parse(native) as {
        status: number;
        state: string;
        body: string;
    };
    expect(response.status).toBe(200);
    expect(response.state).toBe("abc123");
    expect(JSON.parse(response.body)).toEqual({ ok: true });
});

test("handleNativeCallbackRequestUrl parses native request from deep link", async () => {
    const request = {
        url: "storage://app/config",
        headers: {},
        method: "GET",
        body: null,
        state: "state-1",
        callbackUrl: "https://app.example/native-callback",
    };
    const deepLink = new URL("storage://app/config");
    deepLink.searchParams.set("native", JSON.stringify(request));

    const callbackUrl = await handleNativeCallbackRequestUrl(
        deepLink,
        () => new Response("done"),
    );

    const native = callbackUrl.searchParams.get("native");
    expect(native).toBeTruthy();
    if (!native) {
        return;
    }
    expect(JSON.parse(native).state).toBe("state-1");
});

test("handleNativeCallbackRequest maps handler errors to 500 responses", async () => {
    const callbackUrl = await handleNativeCallbackRequest(
        {
            url: "storage://app/config",
            headers: {},
            method: "GET",
            body: null,
            state: "err-state",
            callbackUrl: "https://app.example/native-callback",
        },
        () => {
            throw new Error("boom");
        },
    );

    const nativeParam = callbackUrl.searchParams.get("native");
    expect(nativeParam).toBeTruthy();
    if (!nativeParam) {
        return;
    }

    const response = JSON.parse(nativeParam);
    expect(response.status).toBe(500);
    expect(response.statusText).toBe("boom");
    expect(response.body).toBe("boom");
    expect(response.state).toBe("err-state");
});
