<script lang="ts">
	import '../app.css';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import SearchBar from '$lib/components/search-bar.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { goto } from '$app/navigation';

	async function handleSearch(query: string) {
		goto(`/?query=${encodeURIComponent(query)}`);
	}
</script>

<Sidebar.Provider>
	<div class="flex min-h-screen w-full">
		<!-- 侧边栏 - 添加data属性 -->
		<div data-sidebar="sidebar">
			<AppSidebar />
		</div>

		<!-- 主内容区域 -->
		<Sidebar.Inset class="min-h-screen flex-1">
			<!-- 全局搜索栏 -->
			<div
				class="bg-background/95 supports-[backdrop-filter]:bg-background/60 sticky top-0 z-50 flex h-[73px] w-full items-center border-b backdrop-blur"
			>
				<div class="flex w-full items-center gap-4 px-6">
					<Sidebar.Trigger class="shrink-0" data-sidebar="trigger" />
					<div class="flex-1">
						<SearchBar onSearch={handleSearch} placeholder="搜索视频、UP主..." />
					</div>
				</div>
			</div>

			<slot />
		</Sidebar.Inset>
	</div>
</Sidebar.Provider>
