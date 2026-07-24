<script lang="ts" module>
	import type { Snippet } from 'svelte';

	interface Props {
		title: string;
		description?: string;
		code: string;
		children: Snippet<[]>;
	}
</script>

<script lang="ts">
	import Highlight from 'svelte-highlight';
	import typescript from 'svelte-highlight/languages/typescript';

	let { title, description, code, children }: Props = $props();
	let showCode = $state(false);
	let codeWWithPackageName = $derived(code.replaceAll('$lib', '@aicacia/svelte-forms'));
</script>

<div class="overflow-hidden">
	<div>
		<h3>{title}</h3>
		{#if description}
			<p>{description}</p>
		{/if}
		<hr />
	</div>

	<div>
		{@render children()}
	</div>

	<div class="mt-4 flex flex-row justify-end">
		<button onclick={() => (showCode = !showCode)} class="btn primary">
			{showCode ? 'Hide Code' : 'Show Code'}
		</button>
	</div>

	{#if showCode}
		<div class="max-h-dvh overflow-x-auto">
			<Highlight language={typescript} code={codeWWithPackageName} />
		</div>
	{/if}
</div>
