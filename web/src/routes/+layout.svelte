<script lang="ts">
	import '../app.css';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import SearchBar from '$lib/components/search-bar.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { goto } from '$app/navigation';
	import { appStateStore, setQuery } from '$lib/stores/filter';

	async function handleSearch(query: string) {
		// 更新全局查询状态
		setQuery(query);

		const params = new URLSearchParams();
		if (query.trim()) {
			params.set('query', query);
		}

		// 保持当前的视频源筛选
		const currentState = $appStateStore;
		if (currentState.videoSource.key && currentState.videoSource.value) {
			params.set(currentState.videoSource.key, currentState.videoSource.value);
		}

		const queryString = params.toString();
		const newUrl = queryString ? `/?${queryString}` : '/';
		goto(newUrl);
	}

	// 从全局状态获取当前查询值
	$: searchValue = $appStateStore.query;
</script>

<Sidebar.Provider>
	<div class="flex min-h-screen w-full">
		<div data-sidebar="sidebar">
			<AppSidebar />
		</div>
		<Sidebar.Inset class="min-h-screen flex-1">
			<div
				class="bg-background/95 supports-[backdrop-filter]:bg-background/60 sticky top-0 z-50 flex h-[73px] w-full items-center border-b backdrop-blur"
			>
				<div class="flex w-full items-center gap-4 px-6">
					<Sidebar.Trigger class="shrink-0" data-sidebar="trigger" />
					<div class="flex-1">
						<SearchBar onSearch={handleSearch} value={searchValue} />
					</div>
				</div>
			</div>
			<slot />
		</Sidebar.Inset>
	</div>
</Sidebar.Provider>
