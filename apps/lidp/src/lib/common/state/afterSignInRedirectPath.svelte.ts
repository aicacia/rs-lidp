import { createStorage } from "@aicacia/svelte-headless";
import { goto } from "$app/navigation";
import { resolve } from "$app/paths";

const afterSigninRedirectPath = createStorage<string | null>(
    "after-signin-redirect-path",
    null,
);

export function setAfterSigninRedirectPathFromURL(url: URL) {
    afterSigninRedirectPath.item = url.toString().substring(url.origin.length);
}

export async function afterSigninRedirect() {
    const path = afterSigninRedirectPath.item;
    if (path) {
        afterSigninRedirectPath.item = null;
        await goto(path);
    } else {
        await goto(resolve("/"));
    }
}
