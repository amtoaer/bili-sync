<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import XIcon from '@lucide/svelte/icons/x';

	export let filterTitle: string = '';
	export let filterName: string = '';
	export let onRemove: () => void = () => {};
</script>

{#if filterTitle && filterName}
	<div class="mb-4 flex items-center gap-2">
		<span class="text-muted-foreground text-sm">当前筛选:</span>
		<Badge variant="secondary" class="flex items-center gap-2 pr-1">
			<span>{filterTitle}： {filterName}</span>
			<button
				class="hover:bg-muted-foreground/20 ml-1 cursor-pointer rounded-full p-0.5 transition-colors"
				onclick={onRemove}
				type="button"
			>
				<XIcon class="h-3 w-3" />
			</button>
		</Badge>
	</div>
{/if}
