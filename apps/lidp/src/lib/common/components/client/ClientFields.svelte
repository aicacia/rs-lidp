<script lang="ts" module>
	import type { ClientRegisterRequest } from '$lib/proto/mises';

	export interface ClientFieldsProps {
		client: Partial<ClientRegisterRequest>;
	}
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages';

	let { client }: ClientFieldsProps = $props();
</script>

{#if client.scope}
	{@const scopes = client.scope
		.split(' ')
		.map((s) => s.trim())
		.filter((s) => s.length > 0)}

	<section>
		<h5>{m.authorize_scopes_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each scopes as s (s)}
				<li>{s}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.redirectUris?.length}
	<section>
		<h5>{m.authorize_redirect_uris_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.redirectUris as uri (uri)}
				<li>{uri}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.postLogoutRedirectUris?.length}
	<section>
		<h5>{m.authorize_post_logout_redirect_uris_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.postLogoutRedirectUris as uri (uri)}
				<li>{uri}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.grantTypes?.length}
	<section>
		<h5>{m.authorize_grant_types_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.grantTypes as t (t)}
				<li>{t}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.responseTypes?.length}
	<section>
		<h5>{m.authorize_response_types_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.responseTypes as r (r)}
				<li>{r}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.accessTokenExpiry || client.refreshTokenExpiry}
	<section>
		<h5>{m.authorize_token_expiry_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#if client.accessTokenExpiry}
				<li>{m.authorize_access_token_expires({ seconds: client.accessTokenExpiry })}</li>
			{/if}
			{#if client.refreshTokenExpiry}
				<li>{m.authorize_refresh_token_expires({ seconds: client.refreshTokenExpiry })}</li>
			{/if}
		</ul>
	</section>
{/if}

{#if client.policyUri || client.tosUri}
	<section>
		<h5>{m.authorize_legal_label()}</h5>
		<ul class="space-y-1 text-sm">
			{#if client.policyUri}
				<li>
					<a href={client.policyUri} target="_blank" rel="external"
						>{m.authorize_privacy_policy()}</a
					>
				</li>
			{/if}
			{#if client.tosUri}
				<li>
					<a href={client.tosUri} target="_blank" rel="external">{m.authorize_terms_of_service()}</a
					>
				</li>
			{/if}
		</ul>
	</section>
{/if}
