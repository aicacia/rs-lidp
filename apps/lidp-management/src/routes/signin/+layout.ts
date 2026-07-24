import { resolve } from "$app/paths";
import {
  getLIdpApiUrl,
  validateLIdpApiUrl,
} from "$lib/common/state/lidpClient.svelte";
import { redirect } from "@sveltejs/kit";
import type { LayoutLoad } from "./$types";

export const load: LayoutLoad = async (event) => {
  await event.parent();

  let validLIdpApiUrl = await validateLIdpApiUrl(getLIdpApiUrl());

  if (!validLIdpApiUrl) {
    redirect(302, resolve("/lidp"));
  }
};
