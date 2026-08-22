import { createReturnTo } from "@aicacia/svelte-headless";
import { goto } from "$app/navigation";

export const afterSigninRedirect = createReturnTo({
    id: "after-signin-redirect",
    goto,
    defaultPath: resolve("/"),
});
