<script lang="ts" module>
	import type * as v from 'valibot';

	export interface IssuesProps<V extends v.BaseSchema<unknown, unknown, v.BaseIssue<unknown>>> {
		issues: v.InferIssue<V>[];
	}
</script>

<script lang="ts">
	type V = $$Generic<v.BaseSchema<unknown, unknown, v.BaseIssue<unknown>>>;

	let { issues }: IssuesProps<V> = $props();
</script>

{#if issues.length > 0}
	<ul class="mt-2 space-y-1">
		{#each issues as issue, index (index)}
			<li class="text-sm text-red-500">{issue.message}</li>
		{/each}
	</ul>
{/if}
