<script lang="ts">
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import VideoCard from '$lib/components/video-card.svelte';
	import FilterBadge from '$lib/components/filter-badge.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import api from '$lib/api';
	import type { VideosResponse, VideoSourcesResponse } from '$lib/types';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { videoSourceStore, setVideoSources } from '$lib/stores/video-source';
	import { VIDEO_SOURCES } from '$lib/consts';

	let videosData: VideosResponse | null = null;
	let loading = false;
	let currentPage = 0;
	const pageSize = 20;
	let currentQuery = '';
	let currentFilter: { type: string; id: string } | null = null;

	const breadcrumbItems = [{ href: '/', label: '主页', isActive: true }];

	// 从URL参数获取筛选条件
	function getFilterFromURL(searchParams: URLSearchParams) {
		for (const source of Object.values(VIDEO_SOURCES)) {
			const value = searchParams.get(source.type);
			if (value) {
				return { type: source.type, id: value };
			}
		}
		return null;
	}

	// 获取筛选项名称
	function getFilterName(type: string, id: string): string {
		const videoSources = $videoSourceStore;
		if (!videoSources || !type || !id) return '';

		const sources = videoSources[type as keyof VideoSourcesResponse];
		const source = sources?.find((s) => s.id.toString() === id);
		return source?.name || '';
	}

	// 获取筛选项标题
	function getFilterTitle(type: string): string {
		const sourceConfig = Object.values(VIDEO_SOURCES).find((s) => s.type === type);
		return sourceConfig?.title || '';
	}

	async function loadVideos(
		query?: string,
		pageNum: number = 0,
		filter?: { type: string; id: string } | null
	) {
		loading = true;
		try {
			const params: Record<string, string | number> = {
				page: pageNum,
				page_size: pageSize
			};

			if (query) {
				params.query = query;
			}

			// 添加筛选参数
			if (filter) {
				params[filter.type] = parseInt(filter.id);
			}

			const result = await api.getVideos(params);
			videosData = result.data;
			currentPage = pageNum;
		} catch (error) {
			console.error('加载视频失败:', error);
		} finally {
			loading = false;
		}
	}

	async function loadVideoSources() {
		try {
			const result = await api.getVideoSources();
			setVideoSources(result.data);
		} catch (error) {
			console.error('加载视频来源失败:', error);
		}
	}

	// 从URL加载数据
	async function loadFromURL() {
		const urlQuery = $page.url.searchParams.get('query') || '';
		const urlPage = parseInt($page.url.searchParams.get('page') || '0');
		const urlFilter = getFilterFromURL($page.url.searchParams);

		// 更新本地状态
		currentQuery = urlQuery;
		currentFilter = urlFilter;

		await loadVideos(urlQuery, urlPage, urlFilter);
	}

	async function handlePageChange(pageNum: number) {
		const params = new URLSearchParams($page.url.searchParams);
		if (pageNum > 0) {
			params.set('page', pageNum.toString());
		} else {
			params.delete('page');
		}
		const queryString = params.toString();
		const newUrl = queryString ? `/?${queryString}` : '/';
		goto(newUrl);
	}

	function handleFilterRemove() {
		const params = new URLSearchParams();

		// 保留查询参数
		const query = $page.url.searchParams.get('query');
		if (query) {
			params.set('query', query);
		}

		// 删除所有视频来源筛选参数和页码
		// （页码重置为第一页）

		const queryString = params.toString();
		const newUrl = queryString ? `/?${queryString}` : '/';
		goto(newUrl);
	}

	// 监听URL变化
	$: if ($page.url.searchParams && $videoSourceStore) {
		loadFromURL();
	}

	onMount(async () => {
		// 只加载视频源数据
		if (!$videoSourceStore) {
			await loadVideoSources();
		}
	});

	$: totalPages = videosData ? Math.ceil(videosData.total_count / pageSize) : 0;
	$: filterTitle = currentFilter ? getFilterTitle(currentFilter.type) : '';
	$: filterName = currentFilter ? getFilterName(currentFilter.type, currentFilter.id) : '';
</script>

<!-- 确保容器高度和滚动正确设置 -->
<div class="bg-background min-h-screen w-full">
	<!-- 页面内容 -->
	<div class="w-full px-6 py-6">
		<!-- 面包屑导航 -->
		<div class="mb-6">
			<BreadCrumb items={breadcrumbItems} />
		</div>

		<!-- 筛选条件显示 -->
		<FilterBadge {filterTitle} {filterName} onRemove={handleFilterRemove} />

		<!-- 统计信息 -->
		{#if videosData}
			<div class="mb-6 flex items-center justify-between">
				<div class="text-muted-foreground text-sm">
					共 {videosData.total_count} 个视频
				</div>
				<div class="text-muted-foreground text-sm">
					共 {totalPages} 页
				</div>
			</div>
		{/if}

		<!-- 视频卡片网格 -->
		{#if loading}
			<div class="flex items-center justify-center py-12">
				<div class="text-muted-foreground">加载中...</div>
			</div>
		{:else if videosData?.videos.length}
			<div
				style="display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px; width: 100%; max-width: none; justify-items: start;"
			>
				{#each videosData.videos as video (video.id)}
					<div style="max-width: 400px; width: 100%;">
						<VideoCard {video} />
					</div>
				{/each}
			</div>

			<!-- 翻页组件 -->
			<Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
		{:else}
			<div class="flex items-center justify-center py-12">
				<div class="space-y-2 text-center">
					<p class="text-muted-foreground">暂无视频数据</p>
					<p class="text-muted-foreground text-sm">尝试搜索或检查视频来源配置</p>
				</div>
			</div>
		{/if}
	</div>
</div>
