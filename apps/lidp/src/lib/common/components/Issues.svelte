<script lang="ts" module>
	import type { StandardSchemaV1 } from '@standard-schema/spec';

	export interface IssuesProps<V extends StandardSchemaV1.BaseSchema<unknown, unknown, StandardSchemaV1.BaseIssue<unknown>>> {
		issues: StandardSchemaV1.Issue<V>[];
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
