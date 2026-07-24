<script lang="ts" module>
	export interface AuthorizeProps {
		userInfo: UserInfo;
		authorizeRequest: AuthorizeRequest;
	}
</script>

<script lang="ts">
	import { LoaderCircle } from "@lucide/svelte";
	import { isTauri } from "@tauri-apps/api/core";

	import { goto } from "$app/navigation";
	import { clientClient } from "$lib/common/util/grpcClient";
	import { m } from "$lib/paraglide/messages";
	import type { AuthorizeRequest, Client, UserInfo } from "$lib/proto/mises";
	import AuthorizeClient from "./_AuthorizeClient.svelte";
	import { rejectAuthorizeRequest, resolveAuthorizeRequest } from "./_utils";

	let { userInfo, authorizeRequest }: AuthorizeProps = $props();

	let client = $state<Client | null>(null);

	let loadingClient = $state(true);
	$effect(() => {
		loadingClient = true;
		if (authorizeRequest.clientId === "unknown") {
			console.debug("Client ID is unknown, skipping client info fetch");
			client = null;
			loadingClient = false;
			return;
		}
		console.debug(
			"Fetching client info for clientId",
			authorizeRequest.clientId,
		);
		clientClient()
			.get({ clientId: authorizeRequest.clientId })
			.catch((_e) => null)
			.then((c) => {
				client = c;
			})
			.finally(() => {
				loadingClient = false;
			});
	});

	let loadingUserAllowed = $state(true);
	$effect(() => {
		if (loadingClient) {
			console.debug("Still loading client info, skipping user allowed check");
			return;
		}
		if (!client) {
			console.debug("Client info not available, skipping user allowed check");
			loadingUserAllowed = false;
			return;
		}
		loadingUserAllowed = true;
		console.debug(
			"Checking if user has already allowed this client and scopes",
		);
		clientClient()
			.isAllowedForUser({
				clientId: authorizeRequest.clientId,
				scope: authorizeRequest.scope ?? "",
			})
			.then(onAuthorize)
			.catch(() => {
				// Not yet approved or scopes changed; fall back to consent screen
			})
			.finally(() => {
				loadingUserAllowed = false;
			});
	});

	let loadingAuthorizeRequest = $state(false);
	async function onAuthorize() {
		loadingAuthorizeRequest = true;
		console.debug(
			"Resolving authorize request for clientId",
			authorizeRequest.clientId,
		);
		try {
			await resolveAuthorizeRequest(authorizeRequest);
			if (isTauri()) {
				await goto("/");
			}
		} catch (e) {
			console.error("Error resolving authorize request", e);
		} finally {
			loadingAuthorizeRequest = false;
		}
	}
	async function onAllow() {
		try {
			await clientClient().approveForUser({
				clientId: authorizeRequest.clientId,
			});
			await onAuthorize();
		} catch (e) {
			console.error("Error approving client for user", e);
		}
	}
	async function onDeny() {
		rejectAuthorizeRequest(
			authorizeRequest,
			"access_denied",
			m.authorize_access_denied_reason(),
		);
	}

	let loading = $derived(
		loadingClient || loadingUserAllowed || loadingAuthorizeRequest,
	);
	let disabled = $derived(loading);
</script>

{#if loading}
	<div class="flex flex-row items-center justify-center">
		<LoaderCircle class="animate-spin" />
	</div>
{:else if client}
	<AuthorizeClient {userInfo} {client} {disabled} {onAllow} {onDeny} />
{:else}
	<section>
		<h5>{m.authorize_invalid_request()}</h5>
		<p>{m.authorize_client_not_found()}</p>
		<div class="mt-4 flex flex-row justify-center gap-4">
			<button class="btn secondary" onclick={onDeny}>
				{m.authorize_button_deny()}
			</button>
		</div>
	</section>
{/if}
