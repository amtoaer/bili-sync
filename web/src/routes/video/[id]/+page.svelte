<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import api from '$lib/api';
	import type { ApiError, VideoResponse } from '$lib/types';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { appStateStore, ToQuery } from '$lib/stores/filter';
	import VideoCard from '$lib/components/video-card.svelte';
	import { toast } from 'svelte-sonner';

	let videoData: VideoResponse | null = null;
	let loading = false;
	let error: string | null = null;
	let resetDialogOpen = false;
	let resetting = false;

	async function loadVideoDetail() {
		const videoId = parseInt($page.params.id);
		if (isNaN(videoId)) {
			error = '无效的视频ID';
			toast.error('无效的视频ID');
			return;
		}

		loading = true;
		error = null;

		try {
			const result = await api.getVideo(videoId);
			videoData = result.data;
		} catch (error) {
			console.error('加载视频详情失败:', error);
			toast.error('加载视频详情失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		setBreadcrumb([
			{
				label: '主页',
				onClick: () => {
					goto(`/${ToQuery($appStateStore)}`);
				}
			},
			{ label: '视频详情', isActive: true }
		]);
	});

	// 监听路由参数变化
	$: if ($page.params.id) {
		loadVideoDetail();
	}
</script>

<svelte:head>
	<title>{videoData?.video.name || '视频详情'} - Bili Sync</title>
</svelte:head>

{#if loading}
	<div class="flex items-center justify-center py-12">
		<div class="text-muted-foreground">加载中...</div>
	</div>
{:else if error}
	<div class="flex items-center justify-center py-12">
		<div class="space-y-2 text-center">
			<p class="text-destructive">{error}</p>
			<button
				class="text-muted-foreground hover:text-foreground text-sm transition-colors"
				onclick={() => goto('/')}
			>
				返回首页
			</button>
		</div>
	</div>
{:else if videoData}
	<!-- 视频信息区域 -->
	<section>
		<div class="mb-4 flex items-center justify-between">
			<h2 class="text-xl font-semibold">视频信息</h2>
			<Button
				size="sm"
				variant="outline"
				class="shrink-0"
				onclick={() => (resetDialogOpen = true)}
				disabled={resetting}
			>
				<RotateCcwIcon class="mr-2 h-4 w-4 {resetting ? 'animate-spin' : ''}" />
				重置
			</Button>
		</div>

		<div style="margin-bottom: 1rem;">
			<VideoCard
				video={{
					id: videoData.video.id,
					name: videoData.video.name,
					upper_name: videoData.video.upper_name,
					download_status: videoData.video.download_status
				}}
				mode="detail"
				showActions={false}
				progressHeight="h-3"
				gap="gap-2"
				taskNames={['视频封面', '视频信息', 'UP主头像', 'UP主信息', '分P下载']}
				bind:resetDialogOpen
				bind:resetting
				onReset={async () => {
					try {
						const result = await api.resetVideo((videoData as VideoResponse).video.id);
						if (result.data.resetted) {
							videoData = {
								video: result.data.video,
								pages: result.data.pages
							};
							toast.success('重置成功');
						}
					} catch (error) {
						console.error('重置失败:', error);
						toast.error('重置失败', {
							description: (error as ApiError).message
						});
					}
				}}
			/>
		</div>
	</section>

	<section>
		{#if videoData.pages && videoData.pages.length > 0}
			<div>
				<div class="mb-4 flex items-center justify-between">
					<h2 class="text-xl font-semibold">分页列表</h2>
					<div class="text-muted-foreground text-sm">
						共 {videoData.pages.length} 个分页
					</div>
				</div>

				<div
					class="grid gap-4"
					style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));"
				>
					{#each videoData.pages as pageInfo (pageInfo.id)}
						<VideoCard
							video={{
								id: pageInfo.id,
								name: `P${pageInfo.pid}: ${pageInfo.name}`,
								upper_name: '',
								download_status: pageInfo.download_status
							}}
							mode="page"
							showActions={false}
							customTitle="P{pageInfo.pid}: {pageInfo.name}"
							customSubtitle=""
							taskNames={['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕']}
						/>
					{/each}
				</div>
			</div>
		{:else}
			<div class="py-12 text-center">
				<div class="space-y-2">
					<p class="text-muted-foreground">暂无分P数据</p>
					<p class="text-muted-foreground text-sm">该视频可能为单P视频</p>
				</div>
			</div>
		{/if}
	</section>
{/if}
