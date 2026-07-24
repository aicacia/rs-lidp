import { oidcClient } from "$lib/common/util/grpcClient";
import { redirectToUrl } from "$lib/common/util/redirectToUrl";
import type { AuthorizeRequest } from "$lib/proto/mises";

// helper utilities used by authorization pages

// url helpers

export async function rejectAuthorizeRequest(
    authorizeRequest: Pick<AuthorizeRequest, "redirectUri" | "state" | "nonce">,
    error: string,
    errorDescription: string,
) {
    const url = new URL(authorizeRequest.redirectUri!);
    if (authorizeRequest.state) {
        url.searchParams.set("state", authorizeRequest.state);
    }
    if (authorizeRequest.nonce) {
        url.searchParams.set("nonce", authorizeRequest.nonce);
    }
    url.searchParams.set("error", error);
    url.searchParams.set("error_description", errorDescription);
    await redirectToUrl(url);
}

export async function resolveAuthorizeRequest(
    authorizeRequest: AuthorizeRequest,
) {
    const authorizeResponse = await oidcClient().authorize(authorizeRequest);

    const url = new URL(authorizeResponse.redirectUri!);
    if (authorizeRequest.state) {
        url.searchParams.set("state", authorizeRequest.state);
    }
    if (authorizeRequest.nonce) {
        url.searchParams.set("nonce", authorizeRequest.nonce);
    }

    return await redirectToUrl(url);
    // TODO: change response type to include all possible response parameters, and handle them accordingly
    switch (authorizeRequest.responseMode) {
        case "fragment":
        case "query": {
            switch (authorizeResponse.type) {
                case "authorization_code": {
                    url.searchParams.set("code", authorizeResponse.code);
                    break;
                }
                case "implicit":
                case "hybrid": {
                    url.searchParams.set(
                        "access_token",
                        authorizeResponse.accessToken,
                    );
                    url.searchParams.set(
                        "token_type",
                        authorizeResponse.tokenType,
                    );
                    url.searchParams.set(
                        "expires_in",
                        authorizeResponse.expiresIn,
                    );
                    if (authorizeResponse.idToken) {
                        url.searchParams.set(
                            "id_token",
                            authorizeResponse.idToken,
                        );
                    }
                    break;
                }
            }
            await redirectToUrl(url);
            break;
        }
        case "form_post": {
            throw new Error("not supported yet!");
            break;
        }
        case "web_message": {
            throw new Error("not supported yet!");
            break;
        }
    }
}
