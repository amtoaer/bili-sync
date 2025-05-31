<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import api from '$lib/api';
	import type { VideoResponse } from '$lib/types';
	import UserIcon from '@lucide/svelte/icons/user';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { appStateStore, ToQuery } from '$lib/stores/filter';

	let videoData: VideoResponse | null = null;
	let loading = false;
	let error: string | null = null;
	let resetDialogOpen = false;
	let resetting = false;

	async function loadVideoDetail() {
		const videoId = parseInt($page.params.id);
		if (isNaN(videoId)) {
			error = '无效的视频ID';
			return;
		}

		loading = true;
		error = null;

		try {
			const result = await api.getVideo(videoId);
			videoData = result.data;
		} catch (err) {
			console.error('加载视频详情失败:', err);
			error = '加载视频详情失败';
		} finally {
			loading = false;
		}
	}

	function getTaskName(index: number): string {
		const taskNames = ['视频封面', '视频信息', 'UP主头像', 'UP主信息', '分P下载'];
		return taskNames[index] || `任务${index + 1}`;
	}

	function getPageTaskName(index: number): string {
		const taskNames = ['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕'];
		return taskNames[index] || `任务${index + 1}`;
	}

	function getStatusText(status: number): string {
		if (status === 7) {
			return '已完成';
		} else if (status === 0) {
			return '未开始';
		} else {
			return `失败${status}次`;
		}
	}

	function getSegmentColor(status: number): string {
		if (status === 7) {
			return 'bg-green-500'; // 绿色 - 成功
		} else if (status === 0) {
			return 'bg-yellow-500'; // 黄色 - 未开始
		} else {
			return 'bg-red-500'; // 红色 - 失败
		}
	}

	function getOverallStatus(downloadStatus: [number, number, number, number, number]): {
		text: string;
		color: 'default' | 'secondary' | 'destructive' | 'outline';
	} {
		const completed = downloadStatus.filter((status) => status === 7).length;
		const total = downloadStatus.length;
		const failed = downloadStatus.filter((status) => status !== 7 && status !== 0).length;

		if (completed === total) {
			return { text: '全部完成', color: 'default' };
		} else if (failed > 0) {
			return { text: '部分失败', color: 'destructive' };
		} else {
			return { text: '进行中', color: 'secondary' };
		}
	}

	async function handleReset() {
		if (!videoData) return;

		resetting = true;
		try {
			const result = await api.resetVideo(videoData.video.id);

			// 重置成功后更新本地video数据
			if (result.data.resetted) {
				// 重新加载数据
				await loadVideoDetail();
			}
		} catch (error) {
			console.error('重置失败:', error);
		} finally {
			resetting = false;
			resetDialogOpen = false;
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
		loadVideoDetail();
	});

	// 监听路由参数变化
	$: if ($page.params.id) {
		loadVideoDetail();
	}

	$: videoOverallStatus = videoData ? getOverallStatus(videoData.video.download_status) : null;
	$: videoCompleted = videoData ? videoData.video.download_status.filter((s) => s === 7).length : 0;
	$: videoTotal = videoData ? videoData.video.download_status.length : 0;
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
			{#if videoOverallStatus}
				<Badge variant={videoOverallStatus.color} class="text-sm">
					{videoOverallStatus.text}
				</Badge>
			{/if}
		</div>

		<Card style="margin-bottom: 1rem;">
			<CardHeader class="pb-4">
				<div class="flex items-start justify-between gap-4">
					<div class="min-w-0 flex-1">
						<CardTitle class="mb-2 text-lg leading-tight" title={videoData.video.name}>
							{videoData.video.name}
						</CardTitle>
						<div class="text-muted-foreground flex items-center gap-2 text-sm">
							<UserIcon class="h-4 w-4 shrink-0" />
							<span class="truncate" title={videoData.video.upper_name}>
								{videoData.video.upper_name}
							</span>
						</div>
					</div>
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
			</CardHeader>
			<CardContent class="pt-0">
				<!-- 下载进度 -->
				<div class="space-y-3">
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">下载进度</span>
						<span class="font-medium">{videoCompleted}/{videoTotal}</span>
					</div>

					<!-- 五段进度条 -->
					<div class="flex w-full gap-2">
						{#each videoData.video.download_status as status, index (index)}
							<div
								class="h-3 flex-1 cursor-help rounded-sm transition-all {getSegmentColor(status)}"
								title="{getTaskName(index)}: {getStatusText(status)}"
							></div>
						{/each}
					</div>
				</div>
			</CardContent>
		</Card>
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
						{@const pageOverallStatus = getOverallStatus(pageInfo.download_status)}
						{@const pageCompleted = pageInfo.download_status.filter((s) => s === 7).length}
						{@const pageTotal = pageInfo.download_status.length}

						<Card class="transition-shadow hover:shadow-md">
							<CardHeader class="pb-3">
								<div class="flex items-start justify-between gap-2">
									<CardTitle class="min-w-0 flex-1 text-base leading-tight" title={pageInfo.name}>
										P{pageInfo.pid}: {pageInfo.name}
									</CardTitle>
									<Badge variant={pageOverallStatus.color} class="shrink-0 text-xs">
										{pageOverallStatus.text}
									</Badge>
								</div>
							</CardHeader>
							<CardContent class="pt-0">
								<div class="space-y-3">
									<div class="text-muted-foreground flex justify-between text-xs">
										<span>下载进度</span>
										<span>{pageCompleted}/{pageTotal}</span>
									</div>
									<div class="flex w-full gap-1">
										{#each pageInfo.download_status as status, index (index)}
											<div
												class="h-2 flex-1 cursor-help rounded-sm transition-all {getSegmentColor(
													status
												)}"
												title="{getPageTaskName(index)}: {getStatusText(status)}"
											></div>
										{/each}
									</div>
								</div>
							</CardContent>
						</Card>
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

<!-- 重置确认对话框 -->
<AlertDialog.Root bind:open={resetDialogOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>确认重置</AlertDialog.Title>
			<AlertDialog.Description>
				确定要重置视频 "{videoData?.video.name}"
				的下载状态吗？此操作会将所有失败状态的下载状态重置为未开始，无法撤销。
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>取消</AlertDialog.Cancel>
			<AlertDialog.Action onclick={handleReset} disabled={resetting}>
				{resetting ? '重置中...' : '确认重置'}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
