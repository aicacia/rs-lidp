<script lang="ts" module>
	import type { ClientRegisterRequest } from '$lib/proto/mises';
	import type { ClientRegisterChangedFields } from '$lib/common/util/clientRegisterDiff';

	export interface ClientInfoProps {
		client: Partial<ClientRegisterRequest> & Pick<ClientRegisterRequest, 'logoUri' | 'name'>;
		changedFields: ClientRegisterChangedFields | null;
		disabled?: boolean;
		isNew: boolean;
		onAccept: () => Promise<void>;
		onReject: () => Promise<void>;
	}
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import ClientHeader from './ClientHeader.svelte';
	import ClientFields from './ClientFields.svelte';

	let { client, changedFields, disabled, isNew, onAccept, onReject }: ClientInfoProps = $props();

	let loading = $state(false);

	async function onAcceptInternal() {
		try {
			loading = true;
			await onAccept();
		} finally {
			loading = false;
		}
	}
	async function onRejectInternal() {
		try {
			loading = true;
			await onReject();
		} finally {
			loading = false;
		}
	}
</script>

<ClientHeader {client} />

<hr />

{#if isNew}
	<p>{m.authorize_new_client_request()}</p>
{:else if changedFields}
	<p>{m.authorize_updated_client_request()}</p>
{:else}
	<p>
		{m.authorize_unchanged_client_request()}
	</p>
{/if}

<ClientFields client={isNew ? (client ?? changedFields) : (changedFields ?? {})} />

<hr />

<section>
	<!-- true here until we return the user permissions -->
	{#if true}
		<div class="mt-4 flex flex-row justify-center gap-4">
			<button class="btn secondary" disabled={disabled || loading} onclick={onRejectInternal}
				>{m.client_reject()}</button
			>
			<button class="btn danger" disabled={disabled || loading} onclick={onAcceptInternal}
				>{m.client_accept()}</button
			>
		</div>
	{:else}
		<p>{m.client_not_allowed_to_approve()}</p>
		<div class="mt-4 flex flex-row justify-center gap-4">
			<button class="btn secondary" disabled={disabled || loading} onclick={onRejectInternal}
				>{m.client_close()}</button
			>
		</div>
	{/if}
</section>
