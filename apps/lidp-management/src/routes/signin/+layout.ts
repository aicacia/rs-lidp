import { redirect } from "@sveltejs/kit";
import { resolve } from "$app/paths";
import {
    getManagementApiUrl,
    validateManagementApiUrl,
} from "$lib/common/state/managementClient.svelte";
import type { LayoutLoad } from "./$types";

export const load: LayoutLoad = async (event) => {
    await event.parent();

    const validLIdpApiUrl = await validateManagementApiUrl(
        getManagementApiUrl(),
    );

    if (!validLIdpApiUrl) {
        redirect(302, resolve("/lidp"));
    }
};
