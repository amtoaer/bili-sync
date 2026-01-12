<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import {
		Sheet,
		SheetContent,
		SheetDescription,
		SheetFooter,
		SheetHeader,
		SheetTitle
	} from '$lib/components/ui/sheet/index.js';
	import StatusTaskCard from './status-task-card.svelte';
	import type { VideoInfo, PageInfo, StatusUpdate, UpdateVideoStatusRequest } from '$lib/types';
	import { toast } from 'svelte-sonner';

	let {
		open = $bindable(false),
		video,
		pages = [],
		loading = false,
		onsubmit
	}: {
		open?: boolean;
		video: VideoInfo;
		pages?: PageInfo[];
		loading?: boolean;
		onsubmit: (request: UpdateVideoStatusRequest) => void;
	} = $props();

	// 视频任务名称（与后端 VideoStatus 对应）
	const videoTaskNames = ['视频封面', '视频信息', 'UP 主头像', 'UP 主信息', '分页下载'];

	// 分页任务名称（与后端 PageStatus 对应）
	const pageTaskNames = ['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕'];

	let videoStatuses = $state<number[]>([]);
	let pageStatuses = $state<Record<number, number[]>>({});

	let originalVideoStatuses = $state<number[]>([]);
	let originalPageStatuses = $state<Record<number, number[]>>({});

	$effect(() => {
		videoStatuses = [...video.download_status];
		originalVideoStatuses = [...video.download_status];

		if (pages.length > 0) {
			pageStatuses = pages.reduce(
				(acc, page) => {
					acc[page.id] = [...page.download_status];
					return acc;
				},
				{} as Record<number, number[]>
			);
			originalPageStatuses = pages.reduce(
				(acc, page) => {
					acc[page.id] = [...page.download_status];
					return acc;
				},
				{} as Record<number, number[]>
			);
		} else {
			pageStatuses = {};
			originalPageStatuses = {};
		}
	});

	// 重置单个视频任务到原始状态
	function resetVideoTask(taskIndex: number) {
		videoStatuses[taskIndex] = originalVideoStatuses[taskIndex];
	}

	// 重置单个分页任务到原始状态
	function resetPageTask(pageId: number, taskIndex: number) {
		if (!pageStatuses[pageId]) {
			pageStatuses[pageId] = [];
		}
		pageStatuses[pageId][taskIndex] = originalPageStatuses[pageId]?.[taskIndex] ?? 0;
	}

	function handleVideoStatusChange(taskIndex: number, newValue: number) {
		videoStatuses[taskIndex] = newValue;
	}

	function handlePageStatusChange(pageId: number, taskIndex: number, newValue: number) {
		if (!pageStatuses[pageId]) {
			return;
		}
		pageStatuses[pageId][taskIndex] = newValue;
	}

	function resetAllStatuses() {
		videoStatuses = [...originalVideoStatuses];
		if (pages.length > 0) {
			pageStatuses = pages.reduce(
				(acc, page) => {
					acc[page.id] = [...page.download_status];
					return acc;
				},
				{} as Record<number, number[]>
			);
		} else {
			pageStatuses = {};
		}
	}

	function hasVideoChanges(): boolean {
		return !videoStatuses.every((status, index) => status === originalVideoStatuses[index]);
	}

	function hasPageChanges(): boolean {
		return pages.some((page) => {
			const currentStatuses = pageStatuses[page.id] || [];
			const originalStatuses = originalPageStatuses[page.id] || [];
			return !currentStatuses.every((status, index) => status === originalStatuses[index]);
		});
	}

	// 使用 $derived 创建派生状态
	let hasAnyChanges = $derived(hasVideoChanges() || hasPageChanges());

	function buildRequest(): UpdateVideoStatusRequest {
		const request: UpdateVideoStatusRequest = {};

		request.video_updates = [];
		videoStatuses.forEach((status, index) => {
			if (status !== originalVideoStatuses[index]) {
				request.video_updates!.push({
					status_index: index,
					status_value: status
				});
			}
		});
		request.page_updates = [];
		pages.forEach((page) => {
			const currentStatuses = pageStatuses[page.id] || [];
			const originalStatuses = originalPageStatuses[page.id] || [];
			const updates: StatusUpdate[] = [];

			currentStatuses.forEach((status, index) => {
				if (status !== originalStatuses[index]) {
					updates.push({
						status_index: index,
						status_value: status
					});
				}
			});

			if (updates.length > 0) {
				request.page_updates!.push({
					page_id: page.id,
					updates
				});
			}
		});

		return request;
	}

	function handleSubmit() {
		if (!hasAnyChanges) {
			toast.info('没有状态变更需要提交');
			return;
		}
		const request = buildRequest();
		if (!request.video_updates?.length && !request.page_updates?.length) {
			toast.info('没有状态变更需要提交');
			return;
		}
		onsubmit(request);
	}
</script>

<Sheet bind:open>
	<SheetContent side="right" class="flex w-full flex-col sm:max-w-3xl">
		<SheetHeader class="px-6 pb-2">
			<SheetTitle class="text-lg">编辑状态</SheetTitle>
			<SheetDescription class="text-muted-foreground space-y-1 text-sm">
				<div>自行编辑视频和分页的下载状态。可将任意子任务状态修改为“未开始”或“已完成”。</div>
				<div class="leading-relaxed text-orange-600">
					⚠️ 仅当分页下载状态不是“已完成”时，程序才会尝试执行分页下载。
				</div>
			</SheetDescription>
		</SheetHeader>

		<div class="flex-1 overflow-y-auto px-6">
			<div class="space-y-6 py-2">
				<div>
					<h3 class="mb-4 text-base font-medium">视频状态</h3>
					<div class="bg-card rounded-lg border p-4">
						<div class="space-y-3">
							{#each videoTaskNames as taskName, index (index)}
								<StatusTaskCard
									{taskName}
									currentStatus={videoStatuses[index] ?? 0}
									originalStatus={originalVideoStatuses[index] ?? 0}
									onStatusChange={(newStatus) => handleVideoStatusChange(index, newStatus)}
									onReset={() => resetVideoTask(index)}
									disabled={loading}
								/>
							{/each}
						</div>
					</div>
				</div>

				<!-- 分页状态编辑 -->
				{#if pages.length > 0}
					<div>
						<h3 class="mb-4 text-base font-medium">分页状态</h3>
						<div class="space-y-4">
							{#each pages as page (page.id)}
								<div class="bg-card rounded-lg border">
									<div class="bg-muted/30 border-b px-4 py-3">
										<h4 class="text-sm font-medium">P{page.pid}: {page.name}</h4>
									</div>
									<div class="space-y-3 p-4">
										{#each pageTaskNames as taskName, index (index)}
											<StatusTaskCard
												{taskName}
												currentStatus={(pageStatuses[page.id] || page.download_status)[index] ?? 0}
												originalStatus={originalPageStatuses[page.id]?.[index] ?? 0}
												onStatusChange={(newStatus) =>
													handlePageStatusChange(page.id, index, newStatus)}
												onReset={() => resetPageTask(page.id, index)}
												disabled={loading}
											/>
										{/each}
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		</div>

		<SheetFooter class="bg-background flex gap-2 border-t px-6 pt-4">
			<Button
				variant="outline"
				onclick={resetAllStatuses}
				disabled={!hasAnyChanges}
				class="flex-1 cursor-pointer"
			>
				重置所有状态
			</Button>
			<Button
				onclick={handleSubmit}
				disabled={loading || !hasAnyChanges}
				class="flex-1 cursor-pointer"
			>
				{loading ? '提交中...' : '提交更改'}
			</Button>
		</SheetFooter>
	</SheetContent>
</Sheet>
