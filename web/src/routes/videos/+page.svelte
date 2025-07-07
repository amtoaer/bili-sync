<script lang="ts">
	import VideoCard from '$lib/components/video-card.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import api from '$lib/api';
	import type { VideosResponse, VideoSourcesResponse, ApiError } from '$lib/types';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import HeartIcon from '@lucide/svelte/icons/heart';
	import { goto } from '$app/navigation';
	import { videoSourceStore } from '$lib/stores/video-source';
	import { VIDEO_SOURCES } from '$lib/consts';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { appStateStore, setAll, setCurrentPage, ToQuery } from '$lib/stores/filter';
	import { toast } from 'svelte-sonner';
	import DropdownFilter from '$lib/components/dropdown-filter.svelte';

	const pageSize = 20;

	let videosData: VideosResponse | null = null;
	let loading = false;

	let lastSearch: string | null = null;

	let resetAllDialogOpen = false;
	let resettingAll = false;

	function getApiParams(searchParams: URLSearchParams) {
		let videoSource = null;
		for (const source of Object.values(VIDEO_SOURCES)) {
			const value = searchParams.get(source.type);
			if (value) {
				videoSource = { type: source.type, id: value };
			}
		}
		return {
			query: searchParams.get('query') || '',
			videoSource,
			pageNum: parseInt(searchParams.get('page') || '0')
		};
	}

	function getFilterContent(type: string, id: string) {
		const filterTitle = Object.values(VIDEO_SOURCES).find((s) => s.type === type)?.title || '';
		let filterName = '';
		const videoSources = $videoSourceStore;
		if (videoSources && type && id) {
			const sources = videoSources[type as keyof VideoSourcesResponse];
			filterName = sources?.find((s) => s.id.toString() === id)?.name || '';
		}
		return {
			title: filterTitle,
			name: filterName
		};
	}

	async function loadVideos(
		query: string,
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
			if (filter) {
				params[filter.type] = parseInt(filter.id);
			}
			const result = await api.getVideos(params);
			videosData = result.data;
		} catch (error) {
			console.error('加载视频失败:', error);
			toast.error('加载视频失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	async function handlePageChange(pageNum: number) {
		setCurrentPage(pageNum);
		goto(`/${ToQuery($appStateStore)}`);
	}

	async function handleSearchParamsChange(searchParams: URLSearchParams) {
		const { query, videoSource, pageNum } = getApiParams(searchParams);
		setAll(query, pageNum, videoSource);
		loadVideos(query, pageNum, videoSource);
	}

	async function handleResetVideo(id: number) {
		try {
			const result = await api.resetVideo(id);
			const data = result.data;
			if (data.resetted) {
				toast.success('重置成功', {
					description: `视频「${data.video.name}」已重置`
				});
				const { query, currentPage, videoSource } = $appStateStore;
				await loadVideos(query, currentPage, videoSource);
			} else {
				toast.info('重置无效', {
					description: `视频「${data.video.name}」没有失败的状态，无需重置`
				});
			}
		} catch (error) {
			console.error('重置失败:', error);
			toast.error('重置失败', {
				description: (error as ApiError).message
			});
		}
	}

	async function handleResetAllVideos() {
		resettingAll = true;
		try {
			const result = await api.resetAllVideos();
			const data = result.data;
			if (data.resetted) {
				toast.success('重置成功', {
					description: `已重置 ${data.resetted_videos_count} 个视频和 ${data.resetted_pages_count} 个分页`
				});
				const { query, currentPage, videoSource } = $appStateStore;
				await loadVideos(query, currentPage, videoSource);
			} else {
				toast.info('没有需要重置的视频');
			}
		} catch (error) {
			console.error('重置失败:', error);
			toast.error('重置失败', {
				description: (error as ApiError).message
			});
		} finally {
			resettingAll = false;
			resetAllDialogOpen = false;
		}
	}

	$: if ($page.url.search !== lastSearch) {
		lastSearch = $page.url.search;
		handleSearchParamsChange($page.url.searchParams);
	}

	onMount(async () => {
		setBreadcrumb([
			{
				label: '视频'
			}
		]);
	});

	$: totalPages = videosData ? Math.ceil(videosData.total_count / pageSize) : 0;
	$: filterContent = $appStateStore.videoSource
		? getFilterContent($appStateStore.videoSource.type, $appStateStore.videoSource.id)
		: { title: '', name: '' };
</script>

<svelte:head>
	<title>主页 - Bili Sync</title>
</svelte:head>

<div class="mb-4 flex items-center gap-2">
	<span class="text-sm">当前筛选:</span>
	<DropdownFilter
		title="筛选视频源"
		filters={[
			{
				key: 'videoSource',
				name: '视频来源',
				icon: HeartIcon,
				values: [
					{
						name: '全部',
						id: ''
					}
				]
			}
		]}
		selectedLabel={{
			key: 'videoSource',
			name: '视频来源',
			valueName: filterContent.name || '全部',
			valueId: $appStateStore.videoSource?.id || ''
		}}
	/>
</div>

{#if videosData}
	<div class="mb-6 flex items-center justify-between">
		<div class="flex items-center gap-6">
			<div class=" text-sm font-medium">
				共 {videosData.total_count} 个视频
			</div>
			<div class=" text-sm font-medium">
				共 {totalPages} 页
			</div>
		</div>
		<div class="flex items-center gap-2">
			<Button
				size="sm"
				variant="outline"
				class="hover:bg-accent hover:text-accent-foreground h-8 cursor-pointer text-xs font-medium"
				onclick={() => (resetAllDialogOpen = true)}
				disabled={resettingAll || loading}
			>
				<RotateCcwIcon class="mr-1.5 h-3 w-3 {resettingAll ? 'animate-spin' : ''}" />
				重置所有
			</Button>
		</div>
	</div>
{/if}

{#if loading}
	<div class="flex items-center justify-center py-16">
		<div class="text-muted-foreground/70 text-sm">加载中...</div>
	</div>
{:else if videosData?.videos.length}
	<div
		class="mb-8 grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5"
	>
		{#each videosData.videos as video (video.id)}
			<VideoCard
				{video}
				onReset={async () => {
					await handleResetVideo(video.id);
				}}
			/>
		{/each}
	</div>

	<!-- 翻页组件 -->
	<Pagination
		currentPage={$appStateStore.currentPage}
		{totalPages}
		onPageChange={handlePageChange}
	/>
{:else}
	<div class="flex items-center justify-center py-16">
		<div class="space-y-3 text-center">
			<p class="text-muted-foreground text-sm">暂无视频数据</p>
			<p class="text-muted-foreground/70 text-xs">尝试搜索或检查视频来源配置</p>
		</div>
	</div>
{/if}

<!-- 重置所有视频确认对话框 -->
<AlertDialog.Root bind:open={resetAllDialogOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>重置所有视频</AlertDialog.Title>
			<AlertDialog.Description>
				此操作将重置所有视频和分页的失败状态为未下载状态，使它们在下次下载任务中重新尝试。
				<br />
				<strong class="text-destructive">此操作不可撤销，确定要继续吗？</strong>
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel disabled={resettingAll}>取消</AlertDialog.Cancel>
			<AlertDialog.Action
				onclick={handleResetAllVideos}
				disabled={resettingAll}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				{#if resettingAll}
					<RotateCcwIcon class="mr-2 h-4 w-4 animate-spin" />
					重置中...
				{:else}
					确认重置
				{/if}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
