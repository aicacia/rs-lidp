import { redirect } from "@sveltejs/kit";
import { resolve } from "$app/paths";
import { setAfterSigninRedirectPathFromURL } from "$lib/common/state/afterSignInRedirectPath.svelte";
import { getCurrentUserInfo } from "$lib/common/state/auth.svelte";
import type { LayoutLoad } from "./$types";

export const load: LayoutLoad = async (event) => {
    await event.parent();

    const currentUserInfo = await getCurrentUserInfo();

    if (currentUserInfo) {
        return {
            userInfo: currentUserInfo,
        };
    } else {
        setAfterSigninRedirectPathFromURL(event.url);
        redirect(302, resolve("/signin"));
    }
};
