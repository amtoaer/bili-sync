<script lang="ts">
	import '../app.css';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import SearchBar from '$lib/components/search-bar.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { goto } from '$app/navigation';
	import { appStateStore, setQuery, ToQuery } from '$lib/stores/filter';
	import { Toaster } from '$lib/components/ui/sonner/index.js';
	import { breadcrumbStore } from '$lib/stores/breadcrumb';
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import { videoSourceStore, setVideoSources } from '$lib/stores/video-source';
	import { onMount } from 'svelte';
	import api from '$lib/api';
	import { toast } from 'svelte-sonner';
	import type { ApiError } from '$lib/types';

	let dataLoaded = false;

	async function handleSearch(query: string) {
		setQuery(query);
		goto(`/${ToQuery($appStateStore)}`);
	}

	// 初始化共用数据
	onMount(async () => {
		// 初始化视频源数据，所有组件都会用到
		if (!$videoSourceStore) {
			try {
				const response = await api.getVideoSources();
				setVideoSources(response.data);
			} catch (error) {
				console.error('加载视频来源失败:', error);
				toast.error('加载视频来源失败', {
					description: (error as ApiError).message
				});
			}
		}
		dataLoaded = true;
	});

	// 从全局状态获取当前查询值
	$: searchValue = $appStateStore.query;
</script>

<Toaster />

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
			<div class="bg-background min-h-screen w-full">
				<div class="w-full px-6 py-6">
					{#if $breadcrumbStore.length > 0}
						<div class="mb-6">
							<BreadCrumb items={$breadcrumbStore} />
						</div>
					{/if}
					{#if dataLoaded}
						<slot />
					{/if}
				</div>
			</div>
		</Sidebar.Inset>
	</div>
</Sidebar.Provider>
