<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import type { VideoInfo } from '$lib/types';
	import {
		RotateCcwIcon,
		InfoIcon,
		BrushCleaningIcon,
		UserIcon,
		SquareArrowOutUpRightIcon,
		EllipsisIcon
	} from '@lucide/svelte/icons';
	import { goto } from '$app/navigation';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';

	// 将 bvid 设置为可选属性，但保留 VideoInfo 的其它所有属性
	export let video: Omit<VideoInfo, 'bvid'> & { bvid?: string };
	export let showActions: boolean = true; // 控制是否显示操作按钮
	export let mode: 'default' | 'detail' | 'page' = 'default'; // 卡片模式
	export let customTitle: string = ''; // 自定义标题
	export let customSubtitle: string = ''; // 自定义副标题
	export let taskNames: string[] = []; // 自定义任务名称
	export let showProgress: boolean = true; // 是否显示进度信息
	export let onReset: ((forceReset: boolean) => Promise<void>) | null = null; // 自定义重置函数
	export let onClearAndReset: (() => Promise<void>) | null = null; // 自定义清空重置函数
	export let resetDialogOpen = false; // 导出对话框状态，让父组件可以控制
	export let clearAndResetDialogOpen = false; // 导出清空重置对话框状态
	export let resetting = false;
	export let clearAndResetting = false;

	let forceReset = false;

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
			return 'bg-emerald-500';
		} else if (status === 0) {
			return 'bg-yellow-500';
		} else {
			return 'bg-rose-500';
		}
	}

	function getOverallStatus(
		downloadStatus: number[],
		shouldDownload: boolean
	): {
		text: string;
		style: string;
	} {
		if (!shouldDownload) {
			// 被过滤规则排除，显示为“跳过”
			return { text: '跳过', style: 'bg-gray-100 text-gray-700' };
		}
		const completed = downloadStatus.filter((status) => status === 7).length;
		const total = downloadStatus.length;
		const failed = downloadStatus.filter((status) => status !== 7 && status !== 0).length;

		if (completed === total) {
			// 全部完成，显示为“完成”
			return { text: '完成', style: 'bg-emerald-700 text-emerald-100' };
		} else if (failed > 0) {
			// 出现了失败，显示为“失败”
			return { text: '失败', style: 'bg-rose-700 text-rose-100' };
		} else {
			// 还未开始，显示为“等待”
			return { text: '等待', style: 'bg-yellow-700 text-yellow-100' };
		}
	}

	function getTaskName(index: number): string {
		if (taskNames.length > 0) {
			return taskNames[index] || `任务${index + 1}`;
		}
		const defaultTaskNames = ['视频封面', '视频信息', 'UP主头像', 'UP主信息', '分页下载'];
		return defaultTaskNames[index] || `任务${index + 1}`;
	}

	$: overallStatus = getOverallStatus(video.download_status, video.should_download);
	$: completed = video.download_status.filter((status) => status === 7).length;
	$: total = video.download_status.length;

	async function handleReset() {
		resetting = true;
		if (onReset) {
			await onReset(forceReset);
		}
		resetting = false;
		resetDialogOpen = false;
		forceReset = false;
	}

	async function handleClearAndReset() {
		clearAndResetting = true;
		if (onClearAndReset) {
			await onClearAndReset();
		}
		clearAndResetting = false;
		clearAndResetDialogOpen = false;
	}

	function handleViewDetail() {
		goto(`/video/${video.id}`);
	}

	// 根据模式确定显示的标题和副标题
	$: displayTitle = customTitle || video.name;
	$: displaySubtitle = customSubtitle || video.upper_name;
	$: cardClasses =
		mode === 'default'
			? 'group flex h-full min-w-0 flex-col transition-all hover:shadow-lg hover:shadow-primary/5 border-border/50'
			: 'transition-all hover:shadow-lg border-border/50';
</script>

<Card class={cardClasses}>
	<CardHeader class="shrink-0 pb-3">
		<div class="flex min-w-0 items-start justify-between gap-3">
			<CardTitle
				class="line-clamp-2 min-w-0 flex-1 cursor-default {mode === 'default'
					? 'text-sm'
					: 'text-sm'} leading-relaxed font-medium"
				title={displayTitle}
			>
				{displayTitle}
			</CardTitle>
			<Badge
				variant="secondary"
				class="shrink-0 px-2 py-1 text-xs font-medium {overallStatus.style} "
			>
				{overallStatus.text}
			</Badge>
		</div>
		{#if displaySubtitle}
			<div class="text-muted-foreground mt-1.5 flex min-w-0 items-center gap-1 text-sm">
				<UserIcon class="h-3.5 w-3.5 shrink-0" />
				<span class="min-w-0 cursor-default truncate" title={displaySubtitle}>
					{displaySubtitle}
				</span>
			</div>
		{/if}
	</CardHeader>
	<CardContent
		class={mode === 'default' ? 'flex min-w-0 flex-1 flex-col justify-end pt-0 pb-3' : 'pt-0 pb-4'}
	>
		<div class="space-y-3">
			<!-- 进度条区域 -->
			{#if showProgress}
				<div class="space-y-2">
					<!-- 进度信息 -->
					<div class="text-muted-foreground flex justify-between text-xs font-medium">
						<span class="truncate">下载进度</span>
						<span class="shrink-0">{completed}/{total}</span>
					</div>
					<!-- 进度条 -->
					<div class="flex w-full gap-0.5">
						{#each video.download_status as status, index (index)}
							<Tooltip.Root>
								<Tooltip.Trigger class="flex-1">
									<div
										class="h-1.5 w-full cursor-help rounded-full transition-all {getSegmentColor(
											status
										)}"
									></div>
								</Tooltip.Trigger>
								<Tooltip.Content>
									<p class="text-xs">{getTaskName(index)}: {getStatusText(status)}</p>
								</Tooltip.Content>
							</Tooltip.Root>
						{/each}
					</div>
				</div>
			{/if}

			{#if showActions && mode === 'default'}
				<div class="flex min-w-0 gap-2 pt-1">
					<Button
						size="sm"
						variant="outline"
						class="hover:bg-accent hover:text-accent-foreground h-8 min-w-0 flex-1 cursor-pointer px-3 text-xs font-medium"
						onclick={handleViewDetail}
					>
						<InfoIcon class="mr-1.5 h-3 w-3 shrink-0" />
						<span class="truncate">详情</span>
					</Button>

					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							{#snippet child({ props })}
								<Button
									{...props}
									size="sm"
									variant="outline"
									class="hover:bg-accent hover:text-accent-foreground h-8 shrink-0 cursor-pointer px-2"
								>
									<EllipsisIcon class="h-3 w-3" />
								</Button>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="start" class="w-48">
							<DropdownMenu.Item class="cursor-pointer" onclick={() => (resetDialogOpen = true)}>
								<RotateCcwIcon class="mr-2 h-4 w-4" />
								重置
							</DropdownMenu.Item>
							<DropdownMenu.Item
								class="cursor-pointer"
								onclick={() => (clearAndResetDialogOpen = true)}
							>
								<BrushCleaningIcon class="mr-2 h-4 w-4" />
								清空重置
							</DropdownMenu.Item>
							<DropdownMenu.Item
								class="cursor-pointer"
								onclick={() =>
									window.open(`https://www.bilibili.com/video/${video.bvid}/`, '_blank')}
							>
								<SquareArrowOutUpRightIcon class="mr-2 h-4 w-4" />
								在 B 站打开
							</DropdownMenu.Item>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</div>
			{/if}
		</div>
	</CardContent>
</Card>

<!-- 重置确认对话框 -->
<AlertDialog.Root bind:open={resetDialogOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>重置视频</AlertDialog.Title>
			<AlertDialog.Description>
				确定要重置视频 <strong>"{displayTitle}"</strong> 的下载状态吗？
				<br />
				此操作会将所有的失败状态重置为未开始，<span class="text-destructive font-medium"
					>无法撤销</span
				>。
			</AlertDialog.Description>
		</AlertDialog.Header>

		<div class="space-y-4 py-4">
			<div class="rounded-lg border border-orange-200 bg-orange-50 p-3">
				<div class="mb-2 flex items-center space-x-2">
					<Checkbox id="force-reset-all" bind:checked={forceReset} />
					<Label for="force-reset-all" class="text-sm font-medium text-orange-700"
						>⚠️ 强制重置</Label
					>
				</div>
				<p class="text-xs leading-relaxed text-orange-700">
					除重置失败状态外还会检查修复任务状态的标识位 <br />
					版本升级引入新任务时勾选该选项进行重置，可以允许旧视频执行新任务
				</p>
			</div>
		</div>

		<AlertDialog.Footer>
			<AlertDialog.Cancel
				onclick={() => {
					forceReset = false;
				}}>取消</AlertDialog.Cancel
			>
			<AlertDialog.Action
				onclick={handleReset}
				disabled={resetting}
				class={forceReset ? 'bg-orange-600 hover:bg-orange-700' : ''}
			>
				{resetting ? '重置中...' : forceReset ? '确认强制重置' : '确认重置'}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<!-- 清空重置确认对话框 -->
<AlertDialog.Root bind:open={clearAndResetDialogOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>清空重置视频</AlertDialog.Title>
			<AlertDialog.Description>
				确定要清空重置视频 <strong>"{displayTitle}"</strong> 吗？
				<br />
				<br />
				此操作会：
				<ul class="mt-2 ml-4 list-disc space-y-1">
					<li>将视频状态重置为未开始</li>
					<li>删除所有分页信息</li>
					<li class="text-destructive font-medium">删除视频对应的文件夹</li>
				</ul>
				<br />
				该功能可在多页视频变更后手动触发全量更新，执行后<span class="text-destructive font-medium"
					>无法撤销</span
				>。
			</AlertDialog.Description>
		</AlertDialog.Header>

		<AlertDialog.Footer>
			<AlertDialog.Cancel>取消</AlertDialog.Cancel>
			<AlertDialog.Action
				onclick={handleClearAndReset}
				disabled={clearAndResetting}
				class="bg-destructive hover:bg-destructive/90"
			>
				{clearAndResetting ? '清空重置中...' : '确认清空重置'}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
