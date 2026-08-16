import { Repo } from '@automerge/automerge-repo';
import { IndexedDBStorageAdapter } from '@automerge/automerge-repo-storage-indexeddb';
import { webSocketClientAdapter } from './state/sync.svelte';

export const repo = new Repo({
	// network: [webRTCClientAdapter, webSocketClientAdapter],
	network: [webSocketClientAdapter],
	storage: new IndexedDBStorageAdapter()
});
