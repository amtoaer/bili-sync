<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import * as Input from '$lib/components/ui/input/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	export let placeholder: string = '搜索视频..';
	export let value: string = '';
	export let onSearch: ((query: string) => void) | undefined = undefined;

	function handleSearch() {
		if (onSearch) {
			onSearch(value);
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			handleSearch();
		}
	}
</script>

<div class="flex w-full items-center space-x-2">
	<div class="relative flex-1">
		<SearchIcon class="text-muted-foreground absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2" />
		<Input.Root type="text" {placeholder} bind:value onkeydown={handleKeydown} class="h-11 pl-10" />
	</div>
	<Button onclick={handleSearch} size="default" class="h-11 flex-shrink-0 cursor-pointer px-8"
		>搜索</Button
	>
</div>
