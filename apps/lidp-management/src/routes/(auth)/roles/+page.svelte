<script lang="ts">
  import type { RoleResponse } from "@aicacia/lidp-management-client";
  import { onMount } from "svelte";
  import { lidpManagementApi } from "$lib/common/state/lidpManagementClient.svelte";
  import { notifications } from "$lib/common/state/notifications.svelte";

  const limit = 25;

  let loading = $state(false);
  let creating = $state(false);
  let deletingRoleId = $state<number | null>(null);
  let error = $state<string | null>(null);
  let roles = $state<RoleResponse[]>([]);
  let offset = $state(0);

  let roleName = $state("");
  let roleDescription = $state("");

  function formatTimestamp(value: number): string {
    return new Date(value).toLocaleString();
  }

  async function loadRoles() {
    loading = true;
    error = null;

    try {
      roles = await lidpManagementApi.listRoles({ offset, limit });
    } catch (cause) {
      console.error(cause);
      error = cause instanceof Error ? cause.message : "Failed to load roles";
      notifications.add("Failed to load roles", "error");
    } finally {
      loading = false;
    }
  }

  async function onCreateRole(event: SubmitEvent) {
    event.preventDefault();

    if (!roleName.trim()) {
      notifications.add("Role name is required", "error");
      return;
    }

    creating = true;
    try {
      await lidpManagementApi.createRole({
        createRoleRequest: {
          name: roleName.trim(),
          description: roleDescription.trim() || undefined,
        },
      });
      roleName = "";
      roleDescription = "";
      notifications.add("Role created", "success");
      offset = 0;
      await loadRoles();
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to create role", "error");
    } finally {
      creating = false;
    }
  }

  async function onDeleteRole(role: RoleResponse) {
    if (!confirm(`Delete role "${role.name}"?`)) {
      return;
    }

    deletingRoleId = role.id;
    try {
      await lidpManagementApi.deleteRole({ roleId: role.id });
      notifications.add("Role deleted", "success");
      if (roles.length === 1 && offset > 0) {
        offset -= limit;
      }
      await loadRoles();
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to delete role", "error");
    } finally {
      deletingRoleId = null;
    }
  }

  async function onPrev() {
    if (offset < limit || loading || creating || deletingRoleId !== null) {
      return;
    }

    offset -= limit;
    await loadRoles();
  }

  async function onNext() {
    if (loading || creating || deletingRoleId !== null) {
      return;
    }

    offset += limit;
    await loadRoles();
  }

  onMount(() => {
    void loadRoles();
  });
</script>

<div class="flex flex-col gap-4">
  <div class="flex items-center justify-between">
    <h1 class="mb-0 text-4xl">Roles</h1>
    <p class="mb-0 text-sm opacity-70">Offset: {offset}</p>
  </div>

  <form class="card secondary flex flex-col gap-3" onsubmit={onCreateRole}>
    <h2 class="mb-0 text-2xl">Create role</h2>
    <label class="flex flex-col gap-1">
      <span>Name</span>
      <input bind:value={roleName} type="text" />
    </label>
    <label class="flex flex-col gap-1">
      <span>Description</span>
      <textarea bind:value={roleDescription} rows="3"></textarea>
    </label>
    <div class="flex justify-end">
      <button
        type="submit"
        class="btn primary"
        disabled={creating || loading || deletingRoleId !== null}
        >{creating ? "Creating..." : "Create role"}</button
      >
    </div>
  </form>

  {#if error}
    <div class="card border border-red-700 text-red-200">{error}</div>
  {:else if loading}
    <div class="card">Loading roles...</div>
  {:else if roles.length === 0}
    <div class="card">No roles found.</div>
  {:else}
    <div class="card secondary overflow-x-auto p-0">
      <table class="min-w-full border-collapse text-left">
        <thead>
          <tr class="border-b border-gray-300 dark:border-gray-700">
            <th class="px-4 py-3">ID</th>
            <th class="px-4 py-3">Name</th>
            <th class="px-4 py-3">Description</th>
            <th class="px-4 py-3">Updated</th>
            <th class="px-4 py-3">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each roles as role (role.id)}
            <tr
              class="border-b border-gray-200 align-top last:border-b-0 dark:border-gray-800"
            >
              <td class="px-4 py-3">{role.id}</td>
              <td class="px-4 py-3">{role.name}</td>
              <td class="px-4 py-3">{role.description ?? "-"}</td>
              <td class="px-4 py-3">{formatTimestamp(role.updatedAt)}</td>
              <td class="px-4 py-3">
                <button
                  type="button"
                  class="btn danger"
                  onclick={() => onDeleteRole(role)}
                  disabled={loading || creating || deletingRoleId !== null}
                >
                  {deletingRoleId === role.id ? "Deleting..." : "Delete"}
                </button>
              </td>
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
      disabled={loading || creating || deletingRoleId !== null || offset === 0}
      >Prev</button
    >
    <button
      type="button"
      class="btn secondary"
      onclick={onNext}
      disabled={loading || creating || deletingRoleId !== null}>Next</button
    >
  </div>
</div>
