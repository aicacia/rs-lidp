<script lang="ts">
  import {
    ClientProfile,
    type ClientProfile as ClientProfileValue,
    type ClientRegistration,
    ClientType,
    type ClientType as ClientTypeValue,
    GrantType,
    ResponseType,
    TokenEndpointAuthMethod,
    type TokenEndpointAuthMethod as TokenEndpointAuthMethodValue,
  } from "@aicacia/lidp-management-client";
  import { onMount } from "svelte";
  import { resolve } from "$app/paths";
  import { lidpManagementApi } from "$lib/common/state/lidpManagementClient.svelte";
  import { notifications } from "$lib/common/state/notifications.svelte";

  const limit = 25;
  const profileOptions = Object.values(ClientProfile);
  const clientTypeOptions = Object.values(ClientType);
  const tokenEndpointAuthMethodOptions = Object.values(TokenEndpointAuthMethod);
  const defaultAllowedGrantTypes = [
    GrantType.AuthorizationCode,
    GrantType.RefreshToken,
  ];
  const defaultResponseTypes = [ResponseType.Code];

  let loading = $state(false);
  let creating = $state(false);
  let error = $state<string | null>(null);
  let clients = $state<ClientRegistration[]>([]);
  let offset = $state(0);
  let clientName = $state("");
  let clientUri = $state("");
  let redirectUrisText = $state("");
  let allowedScopesText = $state("");
  let profile = $state<ClientProfileValue>(ClientProfile.WebApplication);
  let clientType = $state<ClientTypeValue>(ClientType.Public);
  let tokenEndpointAuthMethod = $state<TokenEndpointAuthMethodValue>(
    TokenEndpointAuthMethod.None,
  );

  function parseLines(value: string): string[] {
    return value
      .split("\n")
      .map((entry) => entry.trim())
      .filter(Boolean);
  }

  function parseTokens(value: string): string[] {
    return value
      .split(/[\s,]+/)
      .map((entry) => entry.trim())
      .filter(Boolean);
  }

  function resetForm() {
    clientName = "";
    clientUri = "";
    redirectUrisText = "";
    allowedScopesText = "";
    profile = ClientProfile.WebApplication;
    clientType = ClientType.Public;
    tokenEndpointAuthMethod = TokenEndpointAuthMethod.None;
  }

  async function loadClients() {
    loading = true;
    error = null;

    try {
      clients = await lidpManagementApi.listClients({ offset, limit });
    } catch (cause) {
      console.error(cause);
      error = cause instanceof Error ? cause.message : "Failed to load clients";
      notifications.add("Failed to load clients", "error");
    } finally {
      loading = false;
    }
  }

  async function onCreateClient(event: SubmitEvent) {
    event.preventDefault();

    if (!clientName.trim()) {
      notifications.add("Client name is required", "error");
      return;
    }

    creating = true;
    try {
      const createdClient = await lidpManagementApi.createClient({
        clientRegistration: {
          clientName: clientName.trim(),
          clientUri: clientUri.trim() || undefined,
          redirectUris: parseLines(redirectUrisText),
          allowedScopes: parseTokens(allowedScopesText),
          profile,
          clientType,
          tokenEndpointAuthMethod,
          allowedGrantTypes: defaultAllowedGrantTypes,
          responseTypes: defaultResponseTypes,
        },
      });
      resetForm();
      offset = 0;
      await loadClients();
      notifications.add(
        createdClient.clientId
          ? `Client created: ${createdClient.clientId}`
          : "Client created",
        "success",
      );
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to create client", "error");
    } finally {
      creating = false;
    }
  }

  async function onPrev() {
    if (offset < limit || loading || creating) {
      return;
    }
    offset -= limit;
    await loadClients();
  }

  async function onNext() {
    if (loading || creating) {
      return;
    }
    offset += limit;
    await loadClients();
  }

  onMount(() => {
    void loadClients();
  });
</script>

<div class="flex flex-col gap-4">
  <div class="flex items-center justify-between">
    <h1 class="mb-0 text-4xl">Clients</h1>
    <p class="mb-0 text-sm opacity-70">Offset: {offset}</p>
  </div>

  <form class="card secondary flex flex-col gap-3" onsubmit={onCreateClient}>
    <h2 class="mb-0 text-2xl">Create client</h2>
    <label class="flex flex-col gap-1">
      <span>Client name</span>
      <input bind:value={clientName} type="text" required />
    </label>
    <label class="flex flex-col gap-1">
      <span>Client URI</span>
      <input bind:value={clientUri} type="url" />
    </label>
    <label class="flex flex-col gap-1">
      <span>Redirect URIs</span>
      <textarea bind:value={redirectUrisText} rows="4"></textarea>
    </label>
    <label class="flex flex-col gap-1">
      <span>Allowed scopes</span>
      <textarea bind:value={allowedScopesText} rows="3"></textarea>
    </label>
    <div class="grid gap-3 md:grid-cols-3">
      <label class="flex flex-col gap-1">
        <span>Profile</span>
        <select bind:value={profile}>
          {#each profileOptions as option (option)}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </label>
      <label class="flex flex-col gap-1">
        <span>Client type</span>
        <select bind:value={clientType}>
          {#each clientTypeOptions as option (option)}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </label>
      <label class="flex flex-col gap-1">
        <span>Token endpoint auth method</span>
        <select bind:value={tokenEndpointAuthMethod}>
          {#each tokenEndpointAuthMethodOptions as option (option)}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </label>
    </div>
    <div class="flex justify-end">
      <button type="submit" class="btn primary" disabled={creating || loading}>
        {creating ? "Creating..." : "Create client"}
      </button>
    </div>
  </form>

  {#if error}
    <div class="card border border-red-700 text-red-200">{error}</div>
  {:else if loading}
    <div class="card">Loading clients...</div>
  {:else if clients.length === 0}
    <div class="card">No clients found.</div>
  {:else}
    <div class="card secondary overflow-x-auto p-0">
      <table class="min-w-full border-collapse text-left">
        <thead>
          <tr class="border-b border-gray-300 dark:border-gray-700">
            <th class="px-4 py-3">Client ID</th>
            <th class="px-4 py-3">Name</th>
            <th class="px-4 py-3">Profile</th>
            <th class="px-4 py-3">Auth Method</th>
          </tr>
        </thead>
        <tbody>
          {#each clients as client (client.clientId ?? `${client.clientName}-${client.clientUri ?? ""}`)}
            <tr
              class="border-b border-gray-200 align-top last:border-b-0 dark:border-gray-800"
            >
              <td class="px-4 py-3">
                {#if client.clientId}
                  <a href={resolve(`/clients/${client.clientId}`)}
                    >{client.clientId}</a
                  >
                {:else}
                  <span class="disabled">-</span>
                {/if}
              </td>
              <td class="px-4 py-3">{client.clientName}</td>
              <td class="px-4 py-3">{client.profile}</td>
              <td class="px-4 py-3">{client.tokenEndpointAuthMethod}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  <div class="flex items-center justify-end gap-2">
    <button
      type="button"
      class="btn secondary"
      onclick={onPrev}
      disabled={loading || creating || offset === 0}>Prev</button
    >
    <button
      type="button"
      class="btn secondary"
      onclick={onNext}
      disabled={loading || creating}>Next</button
    >
  </div>
</div>
