<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import XIcon from '@lucide/svelte/icons/x';
	import { goto } from '$app/navigation';

	export let filterType: string = '';
	export let filterName: string = '';

	function clearFilter(event: Event) {
		event.preventDefault();
		event.stopPropagation();
		goto('/');
	}

	function getFilterTypeLabel(type: string): string {
		switch (type) {
			case 'collection':
				return '合集 / 列表';
			case 'favorite':
				return '收藏夹';
			case 'submission':
				return '用户投稿';
			case 'watch_later':
				return '稍后再看';
			default:
				return '筛选';
		}
	}
</script>

{#if filterType && filterName}
	<div class="mb-4 flex items-center gap-2">
		<span class="text-muted-foreground text-sm">当前筛选:</span>
		<Badge variant="secondary" class="flex items-center gap-2 pr-1">
			<span>{getFilterTypeLabel(filterType)}: {filterName}</span>
			<button
				class="hover:bg-muted-foreground/20 ml-1 cursor-pointer rounded-full p-0.5 transition-colors"
				onclick={clearFilter}
				type="button"
			>
				<XIcon class="h-3 w-3" />
			</button>
		</Badge>
	</div>
{/if}
