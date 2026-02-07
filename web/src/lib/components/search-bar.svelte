<script lang="ts">
	import { SearchIcon } from '@lucide/svelte/icons';
	import * as Input from '$lib/components/ui/input/index.js';

	export let placeholder: string = '搜索视频..';
	export let value: string = '';
	export let onSearch: ((query: string) => void) | undefined = undefined;
	function handleSearch() {
		onSearch?.(value);
	}
</script>

<div class="flex w-full max-w-48 items-center">
	<div class="relative w-full">
		<SearchIcon class="text-muted-foreground absolute top-1/2 left-2 h-4 w-4 -translate-y-1/2" />
		<Input.Root
			type="text"
			{placeholder}
			bind:value
			class="h-8 w-full border-0 pr-3 pl-7 text-sm shadow-none focus-visible:ring-0"
			onkeydown={(e: KeyboardEvent) => {
				if (e.key === 'Enter') {
					handleSearch();
				}
			}}
		/>
	</div>
</div>
