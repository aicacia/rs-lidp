<script lang="ts" module>
	import type { ClientRegisterRequest } from '$lib/proto/mises';

	export interface ClientHeaderProps {
		client: Partial<ClientRegisterRequest> & Pick<ClientRegisterRequest, 'logoUri' | 'name'>;
	}
</script>

<script lang="ts">
	import Avatar from '$lib/common/components/Avatar.svelte';

	let { client }: ClientHeaderProps = $props();
</script>

<div class="mb-4 flex items-center gap-4">
	{#if client.logoUri}
		<Avatar src={client.logoUri} alt={`${client.name}`} />
	{/if}

	<div>
		<h1 class="text-xl font-semibold">{client.name}</h1>

		{#if client.clientUri}
			<a href={client.clientUri} target="_blank" rel="external">
				{client.clientUri}
			</a>
		{/if}

		{#if client.applicationType}
			<p class="text-sm text-gray-400">{client.applicationType}</p>
		{/if}
	</div>
</div>
