import { redirect } from "@sveltejs/kit";
import { resolve } from "$app/paths";
import { setAfterSigninRedirectPathFromURL } from "$lib/common/state/afterSignInRedirectPath.svelte";
import { getOidcClient } from "$lib/common/state/oidc.svelte";
import type { LayoutLoad } from "./$types";

export const load: LayoutLoad = async (event) => {
    await event.parent();

    try {
        const oidcClient = getOidcClient();
        const currentUserInfo = await oidcClient.getUserInfo();

        if (currentUserInfo) {
            return {
                userInfo: currentUserInfo,
            };
        }
    } catch (error) {
        console.error(error);
        setAfterSigninRedirectPathFromURL(event.url);
        redirect(302, resolve("/signin"));
    }
};
