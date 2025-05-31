<script lang="ts">
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import VideoCard from '$lib/components/video-card.svelte';
	import FilterBadge from '$lib/components/filter-badge.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import api from '$lib/api';
	import type { VideosResponse, VideoSourcesResponse } from '$lib/types';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';

	let searchQuery = '';
	let videosData: VideosResponse | null = null;
	let videoSources: VideoSourcesResponse | null = null;
	let loading = false;
	let currentPage = 0; // 页码从0开始
	const pageSize = 20;

	// 当前筛选状态
	let currentFilter = {
		type: '',
		id: '',
		name: ''
	};

	const breadcrumbItems = [{ href: '/', label: '主页', isActive: true }];

	// 根据URL参数获取筛选条件
	function getFilterFromURL(searchParams: URLSearchParams) {
		const collection = searchParams.get('collection');
		const favorite = searchParams.get('favorite');
		const submission = searchParams.get('submission');
		const watch_later = searchParams.get('watch_later');

		if (collection) return { type: 'collection', id: collection };
		if (favorite) return { type: 'favorite', id: favorite };
		if (submission) return { type: 'submission', id: submission };
		if (watch_later) return { type: 'watch_later', id: watch_later };

		return null;
	}

	// 获取筛选项名称
	function getFilterName(type: string, id: string): string {
		if (!videoSources || !type || !id) return '';

		const sources = videoSources[type as keyof VideoSourcesResponse];
		const source = sources.find((s) => s.id.toString() === id);
		return source?.name || '';
	}

	async function loadVideos(query?: string, pageNum: number = 0) {
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
			const urlFilter = getFilterFromURL($page.url.searchParams);
			if (urlFilter) {
				params[urlFilter.type] = parseInt(urlFilter.id);
				currentFilter = {
					type: urlFilter.type,
					id: urlFilter.id,
					name: getFilterName(urlFilter.type, urlFilter.id)
				};
			} else {
				currentFilter = { type: '', id: '', name: '' };
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
			videoSources = result.data;
		} catch (error) {
			console.error('加载视频来源失败:', error);
		}
	}

	async function handlePageChange(page: number) {
		console.log('翻页到:', page);
		await loadVideos(searchQuery, page);
		// 滚动到页面顶部
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}

	// 监听URL变化（只监听筛选参数变化，不监听搜索）
	$: {
		if ($page.url.searchParams && videoSources) {
			// 只在筛选参数变化时重新加载，保持当前搜索查询
			const urlFilter = getFilterFromURL($page.url.searchParams);
			const hasFilter = !!urlFilter;
			const currentHasFilter = !!(currentFilter.type && currentFilter.id);

			// 只有在筛选状态发生变化时才重新加载
			if (
				hasFilter !== currentHasFilter ||
				(hasFilter &&
					urlFilter &&
					(urlFilter.type !== currentFilter.type || urlFilter.id !== currentFilter.id))
			) {
				loadVideos(searchQuery, 0);
			}
		}
	}

	onMount(async () => {
		await loadVideoSources();

		// 检查URL中是否有搜索查询参数
		const urlQuery = $page.url.searchParams.get('query');
		if (urlQuery) {
			searchQuery = urlQuery;
			await loadVideos(urlQuery, 0);
		} else {
			await loadVideos();
		}
	});

	$: totalPages = videosData ? Math.ceil(videosData.total_count / pageSize) : 0;
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
		<FilterBadge filterType={currentFilter.type} filterName={currentFilter.name} />

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
				style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 16px; width: 100%; max-width: none;"
			>
				{#each videosData.videos as video (video.id)}
					<div style="max-width: 380px;">
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
