<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import {
		Sheet,
		SheetContent,
		SheetDescription,
		SheetFooter,
		SheetHeader,
		SheetTitle
	} from '$lib/components/ui/sheet/index.js';
	import type {
		VideoInfo,
		PageInfo,
		StatusUpdate,
		PageStatusUpdate,
		ResetVideoStatusRequest
	} from '$lib/types';
	import { toast } from 'svelte-sonner';

	export let open = false;
	export let video: VideoInfo;
	export let pages: PageInfo[] = [];
	export let loading = false;

	const dispatch = createEventDispatcher<{
		submit: ResetVideoStatusRequest;
		cancel: void;
	}>();

	// 视频任务名称（与后端 VideoStatus 对应）
	const videoTaskNames = ['视频封面', '视频信息', 'UP主头像', 'UP主信息', '分P下载'];

	// 分页任务名称（与后端 PageStatus 对应）
	const pageTaskNames = ['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕'];

	// 状态选项
	const statusOptions = [
		{ value: 0, label: '未开始', color: 'bg-yellow-500' },
		{ value: 1, label: '失败1次', color: 'bg-red-500' },
		{ value: 2, label: '失败2次', color: 'bg-red-500' },
		{ value: 3, label: '失败3次', color: 'bg-red-500' },
		{ value: 4, label: '失败4次', color: 'bg-red-500' },
		{ value: 7, label: '已完成', color: 'bg-green-500' }
	];

	// 编辑状态
	let videoStatuses = [...video.download_status];
	let pageStatuses: Record<number, number[]> = {};

	// 初始化分页状态
	$: {
		if (pages.length > 0) {
			pageStatuses = pages.reduce(
				(acc, page) => {
					acc[page.id] = [...page.download_status];
					return acc;
				},
				{} as Record<number, number[]>
			);
		}
	}

	function getStatusOption(value: number) {
		return statusOptions.find((opt) => opt.value === value) || statusOptions[0];
	}

	function handleVideoStatusChange(taskIndex: number, newValue: number) {
		videoStatuses[taskIndex] = newValue;
		videoStatuses = [...videoStatuses]; // 触发响应式更新
	}

	function handlePageStatusChange(pageId: number, taskIndex: number, newValue: number) {
		if (!pageStatuses[pageId]) {
			pageStatuses[pageId] = [];
		}
		pageStatuses[pageId][taskIndex] = newValue;
		pageStatuses = { ...pageStatuses }; // 触发响应式更新
	}

	function resetVideoStatuses() {
		videoStatuses = [...video.download_status];
	}

	function resetPageStatuses() {
		pageStatuses = pages.reduce(
			(acc, page) => {
				acc[page.id] = [...page.download_status];
				return acc;
			},
			{} as Record<number, number[]>
		);
	}

	function resetAllStatuses() {
		resetVideoStatuses();
		resetPageStatuses();
	}

	function hasVideoChanges(): boolean {
		return !videoStatuses.every((status, index) => status === video.download_status[index]);
	}

	function hasPageChanges(): boolean {
		return pages.some((page) => {
			const currentStatuses = pageStatuses[page.id] || [];
			return !currentStatuses.every((status, index) => status === page.download_status[index]);
		});
	}

	function hasAnyChanges(): boolean {
		return hasVideoChanges() || hasPageChanges();
	}

	function buildRequest(): ResetVideoStatusRequest {
		const request: ResetVideoStatusRequest = {};

		// 构建视频状态更新
		if (hasVideoChanges()) {
			request.video_updates = [];
			videoStatuses.forEach((status, index) => {
				if (status !== video.download_status[index]) {
					request.video_updates!.push({
						status_index: index,
						status_value: status
					});
				}
			});
		}

		// 构建分页状态更新
		if (hasPageChanges()) {
			request.page_updates = [];
			pages.forEach((page) => {
				const currentStatuses = pageStatuses[page.id] || [];
				const updates: StatusUpdate[] = [];

				currentStatuses.forEach((status, index) => {
					if (status !== page.download_status[index]) {
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
		}

		return request;
	}

	function handleSubmit() {
		if (!hasAnyChanges()) {
			toast.info('没有状态变更需要提交');
			return;
		}

		const request = buildRequest();
		dispatch('submit', request);
	}

	function handleCancel() {
		resetAllStatuses();
		dispatch('cancel');
	}
</script>

<Sheet bind:open>
	<SheetContent side="right" class="w-full overflow-y-auto sm:max-w-4xl">
		<SheetHeader>
			<SheetTitle>编辑状态 - {video.name}</SheetTitle>
			<SheetDescription>
				修改视频和分页的下载状态。可以将失败的任务重置为未开始状态，或者将未完成的任务标记为已完成。
			</SheetDescription>
		</SheetHeader>

		<div class="space-y-6 py-6">
			<!-- 视频状态编辑 -->
			<div>
				<div class="mb-4 flex items-center justify-between">
					<h3 class="text-lg font-semibold">视频状态</h3>
					<Button variant="outline" size="sm" onclick={resetVideoStatuses}>重置视频状态</Button>
				</div>

				<div class="grid gap-4 md:grid-cols-1">
					{#each videoTaskNames as taskName, index}
						<div class="flex items-center justify-between rounded-lg border p-3">
							<div class="flex items-center gap-3">
								<Badge variant="secondary">{index}</Badge>
								<span class="font-medium">{taskName}</span>
							</div>
							<div class="flex items-center gap-2">
								<div
									class="h-3 w-3 rounded-full {getStatusOption(videoStatuses[index]).color}"
								></div>
								<select
									value={videoStatuses[index]}
									on:change={(e) => handleVideoStatusChange(index, parseInt(e.currentTarget.value))}
									class="rounded border px-2 py-1 text-sm"
								>
									{#each statusOptions as option}
										<option value={option.value}>{option.label}</option>
									{/each}
								</select>
							</div>
						</div>
					{/each}
				</div>
			</div>

			<!-- 分页状态编辑 -->
			{#if pages.length > 0}
				<div>
					<div class="mb-4 flex items-center justify-between">
						<h3 class="text-lg font-semibold">分页状态</h3>
						<Button variant="outline" size="sm" onclick={resetPageStatuses}>重置分页状态</Button>
					</div>

					<div class="space-y-4">
						{#each pages as page}
							<div class="rounded-lg border p-4">
								<div class="mb-3">
									<h4 class="font-medium">P{page.pid}: {page.name}</h4>
								</div>

								<div class="grid gap-3 md:grid-cols-1">
									{#each pageTaskNames as taskName, index}
										<div class="flex items-center justify-between rounded border p-2">
											<div class="flex items-center gap-2">
												<Badge variant="outline" class="text-xs">{index}</Badge>
												<span class="text-sm">{taskName}</span>
											</div>
											<div class="flex items-center gap-2">
												<div
													class="h-2 w-2 rounded-full {getStatusOption(
														(pageStatuses[page.id] || page.download_status)[index]
													).color}"
												></div>
												<select
													value={(pageStatuses[page.id] || page.download_status)[index]}
													on:change={(e) =>
														handlePageStatusChange(page.id, index, parseInt(e.currentTarget.value))}
													class="rounded border px-2 py-1 text-xs"
												>
													{#each statusOptions as option}
														<option value={option.value}>{option.label}</option>
													{/each}
												</select>
											</div>
										</div>
									{/each}
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>

		<SheetFooter class="gap-2">
			<Button variant="outline" onclick={resetAllStatuses}>重置全部</Button>
			<Button variant="outline" onclick={handleCancel} disabled={loading}>取消</Button>
			<Button onclick={handleSubmit} disabled={loading || !hasAnyChanges()}>
				{loading ? '提交中...' : '提交更改'}
			</Button>
		</SheetFooter>
	</SheetContent>
</Sheet>
