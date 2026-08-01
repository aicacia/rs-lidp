import { redirect } from "@sveltejs/kit";
import { resolve } from "$app/paths";
import {
  getLidpManagementApiUrl,
  validateLidpManagementApiUrl,
} from "$lib/common/state/lidpManagementClient.svelte";
import type { LayoutLoad } from "./$types";
import {
  getLidpApiUrl,
  validateLidpApiUrl,
} from "$lib/common/state/lidpClient.svelte";

export const ssr = false;
export const prerender = true;

export const load: LayoutLoad = async (event) => {
  await event.parent();

  const [validLidpManagementUrl, validLidpApiUrl] = await Promise.all([
    validateLidpManagementApiUrl(getLidpManagementApiUrl()),
    validateLidpApiUrl(getLidpApiUrl()),
  ]);

  if (!validLidpManagementUrl || !validLidpApiUrl) {
    redirect(302, resolve("/config"));
  }
};
