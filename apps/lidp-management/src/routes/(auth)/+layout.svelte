<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import type { LayoutProps } from "./$types";

  let { children }: LayoutProps = $props();

  const links = [
    { label: "Dashboard", path: "/" },
    { label: "Applications", path: "/applications" },
    { label: "Users", path: "/users" },
  ] as const;

  function isActive(path: string): boolean {
    if (path === "/") {
      return page.url.pathname === "/";
    }
    return (
      page.url.pathname === path || page.url.pathname.startsWith(`${path}/`)
    );
  }

  async function onSignOut() {
    await goto(resolve("/signin"));
  }
</script>

<div class="flex h-full w-full grow overflow-hidden">
  <aside
    class="flex w-64 shrink-0 flex-col gap-3 border-r border-gray-300 bg-white p-4 dark:border-gray-700 dark:bg-gray-950"
  >
    <h2 class="mb-1 text-2xl">Management</h2>
    <nav class="flex flex-col gap-2">
      {#each links as link (link.path)}
        <a
          href={resolve(link.path)}
          class={`btn secondary ${isActive(link.path) ? "active" : ""}`}
          >{link.label}</a
        >
      {/each}
    </nav>
  </aside>

  <section class="flex min-w-0 grow flex-col overflow-hidden">
    <header
      class="flex items-center justify-end border-b border-gray-300 bg-white p-4 dark:border-gray-700 dark:bg-gray-950"
    >
      <button type="button" class="btn danger" onclick={onSignOut}
        >Sign out</button
      >
    </header>
    <div class="min-h-0 grow overflow-auto p-6">
      {@render children()}
    </div>
  </section>
</div>
