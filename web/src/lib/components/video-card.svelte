<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import type { ApiError, VideoInfo } from '$lib/types';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import InfoIcon from '@lucide/svelte/icons/info';
	import UserIcon from '@lucide/svelte/icons/user';
	import { goto } from '$app/navigation';
	import api from '$lib/api';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { toast } from 'svelte-sonner';

	export let video: VideoInfo;
	export let showActions: boolean = true; // 控制是否显示操作按钮
	export let mode: 'default' | 'detail' | 'page' = 'default'; // 卡片模式
	export let customTitle: string = ''; // 自定义标题
	export let customSubtitle: string = ''; // 自定义副标题
	export let taskNames: string[] = []; // 自定义任务名称
	export let showProgress: boolean = true; // 是否显示进度信息
	export let progressHeight: string = 'h-2'; // 进度条高度
	export let gap: string = 'gap-1'; // 进度条间距
	export let onReset: (() => Promise<void>) | null = null; // 自定义重置函数
	export let resetDialogOpen = false; // 导出对话框状态，让父组件可以控制
	export let resetting = false;

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

	function getOverallStatus(downloadStatus: number[]): {
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

	function getTaskName(index: number): string {
		if (taskNames.length > 0) {
			return taskNames[index] || `任务${index + 1}`;
		}
		const defaultTaskNames = ['视频封面', '视频信息', 'UP主头像', 'UP主信息', '分P下载'];
		return defaultTaskNames[index] || `任务${index + 1}`;
	}

	$: overallStatus = getOverallStatus(video.download_status);
	$: completed = video.download_status.filter((status) => status === 7).length;
	$: total = video.download_status.length;

	async function handleReset() {
		resetting = true;
		try {
			if (onReset) {
				await onReset();
			} else {
				await api.resetVideo(video.id);
				window.location.reload();
			}
		} catch (error) {
			console.error('重置失败:', error);
			toast.error('重置失败', {
				description: (error as ApiError).message
			});
		} finally {
			resetting = false;
			resetDialogOpen = false;
		}
	}

	function handleViewDetail() {
		goto(`/video/${video.id}`);
	}

	// 根据模式确定显示的标题和副标题
	$: displayTitle = customTitle || video.name;
	$: displaySubtitle = customSubtitle || video.upper_name;
	$: showUserIcon = mode === 'default';
	$: cardClasses =
		mode === 'default'
			? 'group flex h-full min-w-0 flex-col transition-shadow hover:shadow-md'
			: 'transition-shadow hover:shadow-md';
</script>

<Card class={cardClasses}>
	<CardHeader class={mode === 'default' ? 'flex-shrink-0 pb-3' : 'pb-3'}>
		<div class="flex min-w-0 items-start justify-between gap-2">
			<CardTitle
				class="line-clamp-2 min-w-0 flex-1 cursor-default {mode === 'default'
					? 'text-base'
					: 'text-base'} leading-tight"
				title={displayTitle}
			>
				{displayTitle}
			</CardTitle>
			<Badge variant={overallStatus.color} class="shrink-0 text-xs">
				{overallStatus.text}
			</Badge>
		</div>
		{#if displaySubtitle}
			<div class="text-muted-foreground flex min-w-0 items-center gap-1 text-sm">
				{#if showUserIcon}
					<UserIcon class="h-3 w-3 shrink-0" />
				{/if}
				<span class="min-w-0 cursor-default truncate" title={displaySubtitle}>
					{displaySubtitle}
				</span>
			</div>
		{/if}
	</CardHeader>
	<CardContent
		class={mode === 'default' ? 'flex min-w-0 flex-1 flex-col justify-end pt-0' : 'pt-0'}
	>
		<div class="space-y-3">
			<!-- 进度条区域 -->
			{#if showProgress}
				<div class="space-y-2">
					<div
						class="text-muted-foreground flex justify-between {mode === 'default'
							? 'text-xs'
							: 'text-xs'}"
					>
						<span class="truncate">下载进度</span>
						<span class="shrink-0">{completed}/{total}</span>
					</div>

					<!-- 进度条 -->
					<div class="flex w-full {gap}">
						{#each video.download_status as status, index (index)}
							<Tooltip.Root>
								<Tooltip.Trigger class="flex-1">
									<div
										class="{progressHeight} w-full cursor-help rounded-sm transition-all {getSegmentColor(
											status
										)}"
									></div>
								</Tooltip.Trigger>
								<Tooltip.Content>
									<p>{getTaskName(index)}: {getStatusText(status)}</p>
								</Tooltip.Content>
							</Tooltip.Root>
						{/each}
					</div>
				</div>
			{/if}

			<!-- 操作按钮 -->
			{#if showActions && mode === 'default'}
				<div class="flex min-w-0 gap-1.5">
					<Button
						size="sm"
						variant="outline"
						class="min-w-0 flex-1 cursor-pointer px-2 text-xs"
						onclick={handleViewDetail}
					>
						<InfoIcon class="mr-1 h-3 w-3 shrink-0" />
						<span class="truncate">详情</span>
					</Button>
					<Button
						size="sm"
						variant="outline"
						class="shrink-0 cursor-pointer px-2"
						onclick={() => (resetDialogOpen = true)}
					>
						<RotateCcwIcon class="h-3 w-3" />
					</Button>
				</div>
			{/if}
		</div>
	</CardContent>
</Card>

<!-- 重置确认对话框 -->
<AlertDialog.Root bind:open={resetDialogOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>确认重置</AlertDialog.Title>
			<AlertDialog.Description>
				确定要重置视频 "{displayTitle}"
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
