<script lang="ts">
	import { onMount } from 'svelte';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import VideoItem from '$lib/components/VideoItem.svelte';
	import { listVideos, getVideoSources, deleteVideoSource } from '$lib/api';
	import type { VideoInfo, VideoSourcesResponse } from '$lib/types';
	import Header from '$lib/components/Header.svelte';
	import AddSourceForm from '$lib/components/AddSourceForm.svelte';
	import { toast } from 'svelte-sonner';

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
		'watch_later',
		'bangumi'
	];
	
	// 分类名称映射，显示更友好的中文名称
	const categoryLabels: Record<keyof VideoSourcesResponse, string> = {
		collection: '合集 (Collection)',
		favorite: '收藏夹 (Favorite)',
		submission: 'UP主投稿 (Submission)',
		watch_later: '稍后观看 (Watch Later)',
		bangumi: '番剧 (Bangumi)'
	};
	
	// 分类说明文字
	const categoryDescriptions: Record<keyof VideoSourcesResponse, string> = {
		collection: '视频作者整理的系列视频合集',
		favorite: '您在B站收藏的视频内容',
		submission: 'UP主发布的所有视频',
		watch_later: '添加到稍后观看的视频',
		bangumi: 'B站番剧、电视剧和电影等'
	};

	let activeCategory: keyof VideoSourcesResponse = 'collection';
	let searchQuery = '';
	let videos: VideoInfo[] = [];
	let total = 0;
	let currentPage = 0;
	const pageSize = 10;
	let showAddForm = false; // 控制添加表单的显示

	// 视频列表模型及全局选中模型（只全局允许选中一个）
	let videoListModels: VideoSourcesResponse = {
		collection: [],
		favorite: [],
		submission: [],
		watch_later: [],
		bangumi: []
	};
	// 移除 per 分类选中，新增全局 selectedModel
	let selectedModel: { category: keyof VideoSourcesResponse; id: number } | null = null;
	// 控制侧边栏各分类的折叠状态，true 为折叠
	let collapse: { [key in keyof VideoSourcesResponse]?: boolean } = {
		collection: false,
		favorite: false,
		submission: false,
		watch_later: false,
		bangumi: false
	};

	// 新增：定义 collapse 信号，用于让每个 VideoItem 收起详情
	let videoCollapseSignal = false;

	// 定义视频状态名称和颜色
	const statusNames = [
		'未知', 
		'等待下载',
		'下载中', 
		'已下载', 
		'下载失败',
		'部分P下载失败'
	];
	
	const statusColors = [
		'bg-gray-200', // 未知
		'bg-yellow-200', // 等待下载
		'bg-blue-200', // 下载中
		'bg-green-200', // 已下载
		'bg-red-200', // 下载失败
		'bg-orange-200' // 部分P下载失败
	];

	// 加载视频列表模型
	async function fetchVideoListModels() {
		try {
		videoListModels = await getVideoSources();
			
			// 确保每个分类数组都存在，即使为空
			for (const category of categories) {
				if (!videoListModels[category]) {
					videoListModels[category] = [];
				}
			}
			
		// 默认选中第一个有数据的模型
		for (const key of categories) {
			if (videoListModels[key]?.length) {
				selectedModel = { category: key, id: videoListModels[key][0].id };
				break;
			}
		}
		// 默认使用 activeCategory 对应的选中 id 加载视频
		fetchVideos();
		} catch (error) {
			console.error("获取视频源失败:", error);
			// 初始化空数据结构，确保UI不会崩溃
			videoListModels = {
				collection: [],
				favorite: [],
				submission: [],
				watch_later: [],
				bangumi: []
			};
		}
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

	// 添加视频源成功后的回调
	function handleAddSourceSuccess() {
		showAddForm = false; // 隐藏添加表单
		fetchVideoListModels(); // 刷新视频源列表
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

	// 删除视频源
	async function handleDeleteSource(category: keyof VideoSourcesResponse, id: number, name: string) {
		if (!confirm(`确定要删除视频源 "${name}" 吗？此操作不可撤销。`)) {
			return;
		}
		
		// 询问是否同时删除本地文件
		const deleteLocalFiles = confirm(`是否同时删除本地已下载的文件？\n选择"确定"将删除本地文件，选择"取消"将保留本地文件。`);
		
		try {
			const result = await deleteVideoSource(category, id, deleteLocalFiles);
			if (result.success) {
				toast.success('删除成功', { 
					description: result.message + (deleteLocalFiles ? '，本地文件已删除' : '，本地文件已保留') 
				});
				// 如果删除的是当前选中的视频源，取消选中状态
				if (selectedModel && selectedModel.category === category && selectedModel.id === id) {
					selectedModel = null;
				}
				// 刷新视频源列表
				fetchVideoListModels();
			} else {
				toast.error('删除失败', { description: result.message });
			}
		} catch (error) {
			console.error(error);
			toast.error('删除失败', { description: `错误信息：${error}` });
		}
	}
</script>

<svelte:head>
	<title>bili-sync 管理页</title>
</svelte:head>

<Header>
	<div class="flex">
		<!-- 左侧侧边栏 -->
		<aside class="w-1/4 border-r p-4">
			<div class="flex justify-between items-center mb-4">
				<h2 class="text-xl font-bold">视频来源</h2>
				<Button onclick={() => showAddForm = !showAddForm} class="px-2 py-1 h-auto">
					{showAddForm ? '取消' : '添加'}
				</Button>
			</div>

			{#if showAddForm}
				<div class="mb-4">
					<AddSourceForm onSuccess={handleAddSourceSuccess} />
				</div>
			{/if}
			
			{#each categories as cat}
				<div class="mb-4">
					<!-- 点击标题切换折叠状态 -->
					<button
						class="w-full text-left font-semibold"
						on:click={() => (collapse[cat] = !collapse[cat])}
					>
						{categoryLabels[cat] || cat}
						{collapse[cat] ? '▶' : '▼'}
					</button>
					<!-- 添加分类描述 -->
					<p class="text-xs text-gray-500 mb-1">{categoryDescriptions[cat]}</p>
					{#if !collapse[cat]}
						{#if videoListModels[cat]?.length}
							<ul class="ml-4">
								{#each videoListModels[cat] as model}
									<li class="mb-1 flex items-center">
										<button
											class="flex-grow rounded px-2 py-1 text-left hover:bg-gray-100 {selectedModel &&
											selectedModel.category === cat &&
											selectedModel.id === model.id
												? 'bg-gray-200'
												: ''}"
											on:click={() => selectModel(cat, model.id)}
										>
											{model.name}
										</button>
										<button 
											class="ml-1 text-red-500 hover:text-red-700 px-2" 
											title="删除"
											on:click|stopPropagation={() => handleDeleteSource(cat, model.id, model.name)}
										>
											×
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
			{#if videos.length > 0}
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
			{:else}
				<div class="text-center py-8 text-gray-500">
					无数据，请选择或添加视频源
				</div>
			{/if}
		</main>
	</div>
</Header>
