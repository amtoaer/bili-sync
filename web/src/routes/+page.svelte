<script lang="ts">
	import { onMount } from 'svelte';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import VideoItem from '$lib/components/VideoItem.svelte';
	import { listVideos, getVideoSources } from '$lib/api';
	import type { VideoInfo, VideoSourcesResponse } from '$lib/types';
	import Header from '$lib/components/Header.svelte';

	// API Token 管理
	let apiToken: string = localStorage.getItem('auth_token') || '';
	function updateToken() {
		localStorage.setItem('auth_token', apiToken);
	}

	// 定义分类列表
	const categories: (keyof VideoSourcesResponse)[] = [
		'collection',
		'favorite',
		'submission',
		'watch_later'
	];
	let activeCategory: keyof VideoSourcesResponse = 'collection';
	let searchQuery = '';
	let videos: VideoInfo[] = [];
	let total = 0;
	let currentPage = 0;
	const pageSize = 10;

	// 视频列表模型及全局选中模型（只全局允许选中一个）
	let videoListModels: VideoSourcesResponse = {
		collection: [],
		favorite: [],
		submission: [],
		watch_later: []
	};
	// 移除 per 分类选中，新增全局 selectedModel
	let selectedModel: { category: keyof VideoSourcesResponse; id: number } | null = null;
	// 控制侧边栏各分类的折叠状态，true 为折叠
	let collapse: { [key in keyof VideoSourcesResponse]?: boolean } = {
		collection: false,
		favorite: false,
		submission: false,
		watch_later: false
	};

	// 新增：定义 collapse 信号，用于让每个 VideoItem 收起详情
	let videoCollapseSignal = false;

	// 加载视频列表模型
	async function fetchVideoListModels() {
		videoListModels = await getVideoSources();
		// 默认选中第一个有数据的模型
		for (const key of categories) {
			if (videoListModels[key]?.length) {
				selectedModel = { category: key, id: videoListModels[key][0].id };
				break;
			}
		}
		// 默认使用 activeCategory 对应的选中 id 加载视频
		fetchVideos();
	}

	// 加载视频列表，根据当前 activeCategory 对应的 selectedModel 发起请求
	async function fetchVideos() {
		const params: any = {};
		if (selectedModel && selectedModel.category === activeCategory) {
			params[`${activeCategory}`] = selectedModel.id.toString();
		}
		if (searchQuery) params.query = searchQuery;
		params.page_size = pageSize;
		params.page = currentPage;
		const listRes = await listVideos(params);
		videos = listRes.videos;
		total = listRes.total_count;
	}

	onMount(fetchVideoListModels);

	$: activeCategory, currentPage, searchQuery, fetchVideos();

	function onSearch() {
		currentPage = 0;
		fetchVideos();
	}

	function prevPage() {
		if (currentPage > 0) {
			currentPage -= 1;
			videoCollapseSignal = !videoCollapseSignal;
			fetchVideos();
			// 平滑滚动到顶部
			window.scrollTo({ top: 0, behavior: 'smooth' });
		}
	}

	function nextPage() {
		if ((currentPage + 1) * pageSize < total) {
			currentPage += 1;
			videoCollapseSignal = !videoCollapseSignal;
			fetchVideos();
			// 平滑滚动到顶部
			window.scrollTo({ top: 0, behavior: 'smooth' });
		}
	}

	// 点击侧边栏项时更新 activeCategory 和全局选中模型 id
	function selectModel(category: keyof VideoSourcesResponse, id: number) {
		// 如果当前已选中的模型和点击的一致，则取消筛选
		if (selectedModel && selectedModel.category === category && selectedModel.id === id) {
			selectedModel = null;
		} else {
			selectedModel = { category, id };
		}
		activeCategory = category;
		currentPage = 0;
		videoCollapseSignal = !videoCollapseSignal;
		fetchVideos();
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}
</script>

<svelte:head>
	<title>bili-sync 管理页</title>
</svelte:head>

<Header>
	<div class="flex">
		<!-- 左侧侧边栏 -->
		<aside class="w-1/4 border-r p-4">
			<h2 class="mb-4 text-xl font-bold">视频来源</h2>
			{#each categories as cat}
				<div class="mb-4">
					<!-- 点击标题切换折叠状态 -->
					<button
						class="w-full text-left font-semibold"
						on:click={() => (collapse[cat] = !collapse[cat])}
					>
						{cat}
						{collapse[cat] ? '▶' : '▼'}
					</button>
					{#if !collapse[cat]}
						{#if videoListModels[cat]?.length}
							<ul class="ml-4">
								{#each videoListModels[cat] as model}
									<li class="mb-1">
										<button
											class="w-full rounded px-2 py-1 text-left hover:bg-gray-100 {selectedModel &&
											selectedModel.category === cat &&
											selectedModel.id === model.id
												? 'bg-gray-200'
												: ''}"
											on:click={() => selectModel(cat, model.id)}
										>
											{model.name}
										</button>
									</li>
								{/each}
							</ul>
						{:else}
							<p class="ml-4 text-gray-500">无数据</p>
						{/if}
					{/if}
				</div>
			{/each}
		</aside>

		<!-- 主内容区域 -->
		<main class="flex-1 p-4">
			<div class="mb-4">
				<Input placeholder="搜索视频..." bind:value={searchQuery} on:change={onSearch} />
			</div>
			<div>
				{#each videos as video}
					<VideoItem {video} collapseSignal={videoCollapseSignal} />
				{/each}
			</div>
			<div class="mt-4 flex items-center justify-between">
				<Button onclick={prevPage} disabled={currentPage === 0}>上一页</Button>
				<span>第 {currentPage + 1} 页，共 {Math.ceil(total / pageSize)} 页</span>
				<Button onclick={nextPage} disabled={(currentPage + 1) * pageSize >= total}>下一页</Button>
			</div>
		</main>
	</div>
</Header>
