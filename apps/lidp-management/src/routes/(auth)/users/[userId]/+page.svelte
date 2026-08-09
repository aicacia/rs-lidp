<script lang="ts">
  import type {
    RoleResponse,
    UserConsentResponse,
    UserInfo,
    UserRoleResponse,
  } from "@aicacia/lidp-management-client";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { lidpManagementApi } from "$lib/common/state/lidpManagementClient.svelte";
  import { notifications } from "$lib/common/state/notifications.svelte";

  const parsedUserId = Number(page.params.userId);
  const hasValidUserId = Number.isFinite(parsedUserId);

  let loading = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let resettingPassword = $state(false);
  let error = $state<string | null>(null);
  let user = $state<UserInfo | null>(null);
  let rolesLoading = $state(false);
  let rolesError = $state<string | null>(null);
  let roles = $state<RoleResponse[]>([]);
  let userRolesLoading = $state(false);
  let userRolesError = $state<string | null>(null);
  let userRoles = $state<UserRoleResponse[]>([]);
  let consentsLoading = $state(false);
  let consentsError = $state<string | null>(null);
  let consents = $state<UserConsentResponse[]>([]);
  let selectedRoleId = $state("");
  let assigningRole = $state(false);
  let revokingRoleId = $state<number | null>(null);
  let revokingConsentId = $state<number | null>(null);

  let name = $state("");
  let givenName = $state("");
  let familyName = $state("");
  let email = $state("");
  let locale = $state("");
  let password = $state("");

  const availableRoles = $derived(
    roles.filter(
      (role) => !userRoles.some((userRole) => userRole.roleId === role.id),
    ),
  );

  function syncSelectedRole() {
    if (
      selectedRoleId &&
      !availableRoles.some((role) => role.id === Number(selectedRoleId))
    ) {
      selectedRoleId = "";
    }

    if (!selectedRoleId && availableRoles.length > 0) {
      selectedRoleId = String(availableRoles[0].id);
    }
  }

  function applyUser(currentUser: UserInfo) {
    user = currentUser;
    name = currentUser.name ?? "";
    givenName = currentUser.givenName ?? "";
    familyName = currentUser.familyName ?? "";
    email = currentUser.email ?? "";
    locale = currentUser.locale ?? "";
  }

  async function loadRoles() {
    if (!hasValidUserId) {
      return;
    }

    rolesLoading = true;
    rolesError = null;
    try {
      roles = await lidpManagementApi.listRoles({ offset: 0, limit: 100 });
      syncSelectedRole();
    } catch (cause) {
      console.error(cause);
      rolesError =
        cause instanceof Error ? cause.message : "Failed to load roles";
      notifications.add("Failed to load roles", "error");
    } finally {
      rolesLoading = false;
    }
  }

  async function loadUserRoles() {
    if (!hasValidUserId) {
      return;
    }

    userRolesLoading = true;
    userRolesError = null;
    try {
      userRoles = await lidpManagementApi.listUserRoles({
        userId: parsedUserId,
      });
      syncSelectedRole();
    } catch (cause) {
      console.error(cause);
      userRolesError =
        cause instanceof Error
          ? cause.message
          : "Failed to load assigned roles";
      notifications.add("Failed to load assigned roles", "error");
    } finally {
      userRolesLoading = false;
    }
  }

  async function loadConsents() {
    if (!hasValidUserId) {
      return;
    }

    consentsLoading = true;
    consentsError = null;
    try {
      consents = await lidpManagementApi.listUserConsents({
        userId: parsedUserId,
        offset: 0,
        limit: 100,
      });
    } catch (cause) {
      console.error(cause);
      consentsError =
        cause instanceof Error ? cause.message : "Failed to load user consents";
      notifications.add("Failed to load user consents", "error");
    } finally {
      consentsLoading = false;
    }
  }

  async function loadRelatedData() {
    await Promise.all([loadRoles(), loadUserRoles(), loadConsents()]);
  }

  async function loadUser() {
    if (!hasValidUserId) {
      error = "Invalid user ID";
      return;
    }

    loading = true;
    error = null;
    try {
      const currentUser = await lidpManagementApi.getUser({
        userId: parsedUserId,
      });
      applyUser(currentUser);
      void loadRelatedData();
    } catch (cause) {
      console.error(cause);
      error = cause instanceof Error ? cause.message : "Failed to load user";
      notifications.add("Failed to load user", "error");
    } finally {
      loading = false;
    }
  }

  async function onAssignRole() {
    if (!hasValidUserId || !selectedRoleId) {
      return;
    }

    const roleId = Number(selectedRoleId);
    if (!Number.isFinite(roleId)) {
      notifications.add("Select a valid role", "error");
      return;
    }

    assigningRole = true;
    try {
      await lidpManagementApi.assignRoleToUser({
        userId: parsedUserId,
        roleId,
      });
      notifications.add("Role assigned", "success");
      await Promise.all([loadRoles(), loadUserRoles()]);
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to assign role", "error");
    } finally {
      assigningRole = false;
    }
  }

  async function onRevokeRole(roleId: number) {
    if (!hasValidUserId) {
      return;
    }

    revokingRoleId = roleId;
    try {
      await lidpManagementApi.revokeRoleFromUser({
        userId: parsedUserId,
        roleId,
      });
      notifications.add("Role revoked", "success");
      await Promise.all([loadRoles(), loadUserRoles()]);
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to revoke role", "error");
    } finally {
      revokingRoleId = null;
    }
  }

  async function onRevokeConsent(consent: UserConsentResponse) {
    if (!hasValidUserId) {
      return;
    }

    if (!confirm(`Revoke consent for ${consent.clientId}?`)) {
      return;
    }

    revokingConsentId = consent.id;
    try {
      await lidpManagementApi.revokeUserConsent({
        userId: parsedUserId,
        consentId: consent.id,
      });
      notifications.add("Consent revoked", "success");
      await loadConsents();
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to revoke consent", "error");
    } finally {
      revokingConsentId = null;
    }
  }

  async function onSave() {
    if (!hasValidUserId) {
      return;
    }

    saving = true;
    try {
      await lidpManagementApi.updateUser({
        userId: parsedUserId,
        updateUserRequest: {
          name: name || undefined,
          givenName: givenName || undefined,
          familyName: familyName || undefined,
          email: email || undefined,
          locale: locale || undefined,
        },
      });
      notifications.add("User saved", "success");
      await loadUser();
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to save user", "error");
    } finally {
      saving = false;
    }
  }

  async function onResetPassword() {
    if (!hasValidUserId || !password) {
      return;
    }

    resettingPassword = true;
    try {
      await lidpManagementApi.resetUserPassword({
        userId: parsedUserId,
        resetUserPasswordRequest: {
          password,
        },
      });
      password = "";
      notifications.add("Password reset", "success");
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to reset password", "error");
    } finally {
      resettingPassword = false;
    }
  }

  async function onDeleteUser() {
    if (!hasValidUserId) {
      return;
    }

    if (!confirm("Delete this user?")) {
      return;
    }

    deleting = true;
    try {
      await lidpManagementApi.deleteUser({ userId: parsedUserId });
      notifications.add("User deleted", "success");
      await goto(resolve("/users"));
    } catch (cause) {
      console.error(cause);
      notifications.add("Failed to delete user", "error");
    } finally {
      deleting = false;
    }
  }

  onMount(() => {
    void loadUser();
  });
</script>

<div class="flex flex-col gap-4">
  <h1 class="mb-0 text-4xl">User Detail</h1>

  {#if error}
    <div class="card border border-red-700 text-red-200">{error}</div>
  {:else if loading}
    <div class="card">Loading user...</div>
  {:else if user}
    <div class="flex flex-col gap-4">
      <div class="card secondary flex flex-col gap-3">
        <p class="mb-0 text-sm opacity-70">Sub: {user.sub}</p>
        <label class="flex flex-col gap-1">
          <span>Name</span>
          <input bind:value={name} type="text" />
        </label>
        <label class="flex flex-col gap-1">
          <span>Given name</span>
          <input bind:value={givenName} type="text" />
        </label>
        <label class="flex flex-col gap-1">
          <span>Family name</span>
          <input bind:value={familyName} type="text" />
        </label>
        <label class="flex flex-col gap-1">
          <span>Email</span>
          <input bind:value={email} type="email" />
        </label>
        <label class="flex flex-col gap-1">
          <span>Locale</span>
          <input bind:value={locale} type="text" />
        </label>
        <div class="flex justify-end">
          <button
            type="button"
            class="btn primary"
            onclick={onSave}
            disabled={saving || deleting || resettingPassword}
            >{saving ? "Saving..." : "Save"}</button
          >
        </div>
      </div>

      <div class="card secondary flex flex-col gap-3">
        <h2 class="mb-0 text-2xl">Reset password</h2>
        <label class="flex flex-col gap-1">
          <span>New password</span>
          <input bind:value={password} type="password" />
        </label>
        <div class="flex justify-end">
          <button
            type="button"
            class="btn secondary"
            onclick={onResetPassword}
            disabled={!password || resettingPassword || saving || deleting}
            >{resettingPassword ? "Resetting..." : "Reset password"}</button
          >
        </div>
      </div>

      <div class="card secondary flex flex-col gap-3">
        <h2 class="mb-0 text-2xl">Roles</h2>
        <div class="flex flex-col gap-3 md:flex-row md:items-end">
          <label class="flex grow flex-col gap-1">
            <span>Assign role</span>
            <select
              bind:value={selectedRoleId}
              disabled={rolesLoading ||
                userRolesLoading ||
                assigningRole ||
                revokingRoleId !== null}
            >
              <option value="">Select role</option>
              {#each availableRoles as role (role.id)}
                <option value={String(role.id)}>{role.name}</option>
              {/each}
            </select>
          </label>
          <button
            type="button"
            class="btn primary"
            onclick={onAssignRole}
            disabled={!selectedRoleId ||
              rolesLoading ||
              userRolesLoading ||
              assigningRole ||
              revokingRoleId !== null}
          >
            {assigningRole ? "Assigning..." : "Assign role"}
          </button>
        </div>

        {#if rolesError}
          <div class="card border border-red-700 text-red-200">
            {rolesError}
          </div>
        {:else if userRolesError}
          <div class="card border border-red-700 text-red-200">
            {userRolesError}
          </div>
        {:else if rolesLoading || userRolesLoading}
          <div class="card">Loading roles...</div>
        {:else if userRoles.length === 0}
          <div class="card">No roles assigned.</div>
        {:else}
          <div class="overflow-x-auto">
            <table class="min-w-full border-collapse text-left">
              <thead>
                <tr class="border-b border-gray-300 dark:border-gray-700">
                  <th class="px-4 py-3">Role ID</th>
                  <th class="px-4 py-3">Name</th>
                  <th class="px-4 py-3">Actions</th>
                </tr>
              </thead>
              <tbody>
                {#each userRoles as userRole (userRole.id)}
                  <tr
                    class="border-b border-gray-200 align-top last:border-b-0 dark:border-gray-800"
                  >
                    <td class="px-4 py-3">{userRole.roleId}</td>
                    <td class="px-4 py-3">{userRole.roleName}</td>
                    <td class="px-4 py-3">
                      <button
                        type="button"
                        class="btn danger"
                        onclick={() => onRevokeRole(userRole.roleId)}
                        disabled={assigningRole || revokingRoleId !== null}
                      >
                        {revokingRoleId === userRole.roleId
                          ? "Revoking..."
                          : "Revoke"}
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>

      <div class="card secondary flex flex-col gap-3">
        <h2 class="mb-0 text-2xl">Consents</h2>
        {#if consentsError}
          <div class="card border border-red-700 text-red-200">
            {consentsError}
          </div>
        {:else if consentsLoading}
          <div class="card">Loading consents...</div>
        {:else if consents.length === 0}
          <div class="card">No consents found.</div>
        {:else}
          <div class="overflow-x-auto">
            <table class="min-w-full border-collapse text-left">
              <thead>
                <tr class="border-b border-gray-300 dark:border-gray-700">
                  <th class="px-4 py-3">Client ID</th>
                  <th class="px-4 py-3">Scope</th>
                  <th class="px-4 py-3">Redirect URI</th>
                  <th class="px-4 py-3">Actions</th>
                </tr>
              </thead>
              <tbody>
                {#each consents as consent (consent.id)}
                  <tr
                    class="border-b border-gray-200 align-top last:border-b-0 dark:border-gray-800"
                  >
                    <td class="px-4 py-3">{consent.clientId}</td>
                    <td class="px-4 py-3">{consent.scope}</td>
                    <td class="px-4 py-3 break-all">{consent.redirectUri}</td>
                    <td class="px-4 py-3">
                      <button
                        type="button"
                        class="btn danger"
                        onclick={() => onRevokeConsent(consent)}
                        disabled={revokingConsentId !== null}
                      >
                        {revokingConsentId === consent.id
                          ? "Revoking..."
                          : "Revoke"}
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>

      <div class="card secondary flex flex-col gap-3">
        <h2 class="mb-0 text-2xl">Danger zone</h2>
        <p class="mb-0 text-sm opacity-80">
          Deleting this user cannot be undone.
        </p>
        <div class="flex justify-end">
          <button
            type="button"
            class="btn danger"
            onclick={onDeleteUser}
            disabled={deleting || saving || resettingPassword}
            >{deleting ? "Deleting..." : "Delete user"}</button
          >
        </div>
      </div>
    </div>
  {/if}
</div>
