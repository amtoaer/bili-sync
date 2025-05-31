<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import type { VideoInfo } from '$lib/types';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import PlayIcon from '@lucide/svelte/icons/play';
	import UserIcon from '@lucide/svelte/icons/user';
	import { goto } from '$app/navigation';
	import api from '$lib/api';

	export let video: VideoInfo;
	export let showActions: boolean = true; // 控制是否显示操作按钮

	let resetDialogOpen = false;
	let resetting = false;

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
		const taskNames = ['视频封面', '视频信息', 'UP主头像', 'UP主信息', '分P下载'];
		return taskNames[index] || `任务${index + 1}`;
	}

	$: overallStatus = getOverallStatus(video.download_status);
	$: completed = video.download_status.filter((status) => status === 7).length;
	$: total = video.download_status.length;

	async function handleReset() {
		resetting = true;
		try {
			await api.resetVideo(video.id);
			// 重置成功后可以刷新页面或更新状态
			window.location.reload();
		} catch (error) {
			console.error('重置失败:', error);
			// 这里可以添加错误提示
		} finally {
			resetting = false;
			resetDialogOpen = false;
		}
	}

	function handleViewDetail() {
		goto(`/video/${video.id}`);
	}
</script>

<Card class="group flex h-full min-w-0 flex-col transition-shadow hover:shadow-md">
	<CardHeader class="flex-shrink-0 pb-3">
		<div class="flex min-w-0 items-start justify-between gap-2">
			<CardTitle
				class="line-clamp-2 min-w-0 flex-1 cursor-default text-base leading-tight"
				title={video.name}
			>
				{video.name}
			</CardTitle>
			<Badge variant={overallStatus.color} class="shrink-0 text-xs">
				{overallStatus.text}
			</Badge>
		</div>
		<div class="text-muted-foreground flex min-w-0 items-center gap-1 text-sm">
			<UserIcon class="h-3 w-3 shrink-0" />
			<span class="min-w-0 cursor-default truncate" title={video.upper_name}
				>{video.upper_name}</span
			>
		</div>
	</CardHeader>
	<CardContent class="flex min-w-0 flex-1 flex-col justify-end pt-0">
		<div class="space-y-3">
			<!-- 五段进度条 -->
			<div class="space-y-2">
				<div class="text-muted-foreground flex justify-between text-xs">
					<span class="truncate">下载进度</span>
					<span class="shrink-0">{completed}/{total}</span>
				</div>

				<!-- 五段进度条 -->
				<div class="flex w-full gap-1">
					{#each video.download_status as status, index (index)}
						<div
							class="h-2 flex-1 cursor-help rounded-sm transition-all {getSegmentColor(status)}"
							title="{getTaskName(index)}: {getStatusText(status)}"
						></div>
					{/each}
				</div>
			</div>

			<!-- 操作按钮 -->
			{#if showActions}
				<div class="flex min-w-0 gap-1.5">
					<Button
						size="sm"
						variant="outline"
						class="min-w-0 flex-1 cursor-pointer px-2 text-xs"
						onclick={handleViewDetail}
					>
						<PlayIcon class="mr-1 h-3 w-3 shrink-0" />
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
				确定要重置视频 "{video.name}" 的下载状态吗？此操作将清除所有失败的下载状态，无法撤销。
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
