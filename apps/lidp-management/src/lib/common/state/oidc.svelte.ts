import { OidcClient } from "@aicacia/oidc-client";
import { isTauri } from "@tauri-apps/api/core";
import { env } from "$env/dynamic/public";
import icon256x256Png from "$lib/assets/icon256x256.png";
import { getLIdpApiUrl } from "./lidpClient.svelte";

const oidcClient = $derived.by(
    () =>
        new OidcClient({
            clientConfig: {
                authority: getLIdpApiUrl()?.toString() ?? "lidp://app",
                redirectUri: `${env.PUBLIC_URL}/callback`,
                clientId: isTauri()
                    ? "lidp-management-desktop"
                    : "lidp-management-web",
                responseType: "code",
                registration: {
                    clientId: "mises-simple-example",
                    clientName: "Simple Example",
                    scope: "openid profile address offline email phone",
                    redirectUris: [`${env.PUBLIC_URL}/callback`],
                    postLogoutRedirectUris: [`${env.PUBLIC_URL}/logout`],
                    logoUri: `${env.PUBLIC_URL}${icon256x256Png}`,
                    clientUri: `${env.PUBLIC_URL}`,
                    policyUri: `${env.PUBLIC_URL}/policy`,
                    tosUri: `${env.PUBLIC_URL}/terms`,
                    profile: "web_application",
                    clientType: "public",
                    tokenEndpointAuthMethod: "none",
                    grantTypes: ["authorization_code", "refresh_token"],
                    responseTypes: ["code"],
                    accessTokenExpiry: 3600,
                    refreshTokenExpiry: 604800,
                },
            },
        }),
);

export function getOidcClient() {
    return oidcClient;
}

export async function signin() {
    const client = oidcClient;
    return await client.signin();
}
