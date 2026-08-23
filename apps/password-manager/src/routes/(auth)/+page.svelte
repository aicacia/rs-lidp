<script lang="ts">
	import {
		getStorageBridgeUrl,
		getStorageClient,
		loadStorageBridgeConfig,
		resetStorageBridgeConfig
	} from '$lib/common/state/storageConfig.svelte';
	import {
		runStorageFileExample,
		type StorageFileExampleState
	} from '$lib/common/util/storageFileExample';

	let exampleState: StorageFileExampleState | null = $state(null);
	let isLoading = $state(false);
	let connectionError: string | null = $state(null);
	let bridgeUrl = $derived(getStorageBridgeUrl());

	async function connectToBridge(): Promise<void> {
		isLoading = true;
		connectionError = null;
		exampleState = null;

		try {
			await loadStorageBridgeConfig();

			if (!getStorageBridgeUrl()) {
				connectionError = 'Storage bridge URL not found. Make sure the storage app is running.';
				isLoading = false;
				return;
			}

			// Run the example after successful connection
			await runExample();
		} catch (error) {
			connectionError =
				error instanceof Error ? error.message : 'Failed to connect to storage bridge';
			isLoading = false;
		}
	}

	async function runExample(): Promise<void> {
		isLoading = true;
		connectionError = null;
		exampleState = null;

		try {
			const client = getStorageClient();
			exampleState = await runStorageFileExample(client);
		} catch (error) {
			exampleState = {
				status: 'error',
				message: error instanceof Error ? error.message : 'Failed to run example'
			};
		} finally {
			isLoading = false;
		}
	}

	function disconnectFromBridge(): void {
		resetStorageBridgeConfig();
		exampleState = null;
		connectionError = null;
		isLoading = false;
	}
</script>

<div class="flex flex-col grow items-center justify-center gap-6">
	<h1>Welcome</h1>

	{#if connectionError}
		<div class="text-red-600 text-center max-w-md p-4 rounded border border-red-600">
			<p class="font-semibold">Connection Error</p>
			<p class="text-sm">{connectionError}</p>
			<button
				onclick={connectToBridge}
				disabled={isLoading}
				class="mt-4 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
			>
				{isLoading ? 'Connecting...' : 'Try Again'}
			</button>
		</div>
	{:else if !bridgeUrl}
		<div class="text-center max-w-md">
			<p class="text-gray-600 mb-4">Connect to the storage bridge to enable file operations.</p>
			<button
				onclick={connectToBridge}
				disabled={isLoading}
				class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
			>
				{isLoading ? 'Connecting...' : 'Connect to Storage Bridge'}
			</button>
		</div>
	{:else if isLoading}
		<div class="text-blue-600 text-center">
			<p class="font-semibold">Running storage file demo...</p>
		</div>
	{:else if exampleState}
		<div
			class="text-center max-w-md p-4 rounded border"
			class:border-green-600={exampleState.status === 'success'}
			class:border-red-600={exampleState.status === 'error'}
		>
			<p
				class="font-semibold"
				class:text-green-600={exampleState.status === 'success'}
				class:text-red-600={exampleState.status === 'error'}
			>
				{exampleState.status === 'success' ? '✓' : '✗'}
				{exampleState.message}
			</p>
			{#if exampleState.content}
				<p class="text-sm mt-2">
					<strong>File content:</strong>
					<span class="font-mono text-xs break-all">{exampleState.content}</span>
				</p>
			{/if}
			<div class="mt-4 flex gap-2 justify-center">
				<button
					onclick={runExample}
					class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
				>
					Run Again
				</button>
				<button
					onclick={disconnectFromBridge}
					class="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700"
				>
					Disconnect
				</button>
			</div>
		</div>
	{:else}
		<div class="text-center max-w-md p-4 rounded border border-gray-300 bg-gray-50">
			<p class="font-semibold text-gray-800">Storage bridge connected</p>
			{#if bridgeUrl}
				<p class="mt-2 break-all text-sm text-gray-600">{bridgeUrl}</p>
			{/if}
			<div class="mt-4 flex gap-2 justify-center">
				<button
					onclick={runExample}
					disabled={isLoading}
					class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
				>
					{isLoading ? 'Running...' : 'Run Storage Demo'}
				</button>
				<button
					onclick={disconnectFromBridge}
					class="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700"
				>
					Disconnect
				</button>
			</div>
		</div>
	{/if}
</div>
