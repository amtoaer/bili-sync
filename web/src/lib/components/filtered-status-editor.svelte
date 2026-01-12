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
	import type { StatusUpdate, UpdateFilteredVideoStatusRequest } from '$lib/types';
	import { toast } from 'svelte-sonner';

	let {
		open = $bindable(false),
		hasFilters = false,
		loading = false,
		filterDescriptionParts = [],
		onsubmit
	}: {
		open?: boolean;
		hasFilters?: boolean;
		loading?: boolean;
		filterDescriptionParts?: string[];
		onsubmit: (request: UpdateFilteredVideoStatusRequest) => void;
	} = $props();

	// 视频任务名称（与后端 VideoStatus 对应）
	const videoTaskNames = ['视频封面', '视频信息', 'UP 主头像', 'UP 主信息', '分页下载'];

	// 分页任务名称（与后端 PageStatus 对应）
	const pageTaskNames = ['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕'];

	// 状态选项：null 表示未选择，0 表示未开始，7 表示已完成
	type StatusValue = null | 0 | 7;

	// 视频任务状态，默认都是 null（未选择）
	let videoStatuses = $state<StatusValue[]>(Array(5).fill(null));

	// 分页任务状态，默认都是 null（未选择）
	let pageStatuses = $state<StatusValue[]>(Array(5).fill(null));

	function setVideoStatus(taskIndex: number, value: StatusValue) {
		videoStatuses[taskIndex] = value;
	}

	function setPageStatus(taskIndex: number, value: StatusValue) {
		pageStatuses[taskIndex] = value;
	}

	function resetVideoStatus(taskIndex: number) {
		videoStatuses[taskIndex] = null;
	}

	function resetPageStatus(taskIndex: number) {
		pageStatuses[taskIndex] = null;
	}

	function resetAllStatuses() {
		videoStatuses = Array(5).fill(null);
		pageStatuses = Array(5).fill(null);
	}

	function hasVideoChanges(): boolean {
		return videoStatuses.some((status) => status !== null);
	}

	function hasPageChanges(): boolean {
		return pageStatuses.some((status) => status !== null);
	}

	let hasAnyChanges = $derived(hasVideoChanges() || hasPageChanges());

	function buildRequest(): UpdateFilteredVideoStatusRequest {
		const request: UpdateFilteredVideoStatusRequest = {};

		// 添加视频更新
		const videoUpdates: StatusUpdate[] = [];
		videoStatuses.forEach((status, index) => {
			if (status !== null) {
				videoUpdates.push({
					status_index: index,
					status_value: status
				});
			}
		});
		if (videoUpdates.length > 0) {
			request.video_updates = videoUpdates;
		}

		// 添加分页更新
		const pageUpdates: StatusUpdate[] = [];
		pageStatuses.forEach((status, index) => {
			if (status !== null) {
				pageUpdates.push({
					status_index: index,
					status_value: status
				});
			}
		});
		if (pageUpdates.length > 0) {
			request.page_updates = pageUpdates;
		}

		return request;
	}

	function handleSubmit() {
		if (!hasAnyChanges) {
			toast.info('请至少选择一个状态进行修改');
			return;
		}
		const request = buildRequest();
		onsubmit(request);
	}

	// 当 Sheet 关闭时重置状态
	$effect(() => {
		if (!open) {
			resetAllStatuses();
		}
	});

	function getStatusInfo(status: StatusValue) {
		if (status === 0) {
			return { label: '未开始', class: 'text-yellow-600', dotClass: 'bg-yellow-600' };
		}
		if (status === 7) {
			return { label: '已完成', class: 'text-emerald-600', dotClass: 'bg-emerald-600' };
		}
		return { label: '无修改', class: 'text-muted-foreground', dotClass: 'bg-muted-foreground' };
	}
</script>

<Sheet bind:open>
	<SheetContent side="right" class="flex w-full flex-col sm:max-w-3xl">
		<SheetHeader class="px-6 pb-2">
			<SheetTitle class="text-lg">{hasFilters ? '编辑筛选视频' : '编辑全部视频'}</SheetTitle>
			<SheetDescription class="text-muted-foreground space-y-2 text-sm"
				>批量编辑视频和分页的下载状态。可将任意子任务状态修改为“未开始”或“已完成”。<br />
				{#if hasFilters}
					正在编辑<strong>符合以下筛选条件</strong>的视频的下载状态：
					<div class="bg-muted my-2 rounded-md p-2 text-left">
						{#each filterDescriptionParts as part, index (index)}
							<div><strong>{part}</strong></div>
						{/each}
					</div>
				{:else}
					正在编辑<strong>全部视频</strong>的下载状态。 <br />
				{/if}
				<div class="leading-relaxed text-orange-600">
					⚠️ 仅当分页下载状态不是"已完成"时，程序才会尝试执行分页下载。
				</div>
			</SheetDescription>
		</SheetHeader>

		<div class="flex-1 overflow-y-auto px-6">
			<div class="space-y-6 py-2">
				<!-- 视频状态编辑 -->
				<div>
					<h3 class="mb-4 text-base font-medium">视频状态</h3>
					<div class="bg-card rounded-lg border p-4">
						<div class="space-y-3">
							{#each videoTaskNames as taskName, index (index)}
								{@const statusInfo = getStatusInfo(videoStatuses[index])}
								{@const isModified = videoStatuses[index] !== null}
								<div
									class="bg-background hover:bg-muted/30 flex items-center justify-between rounded-md border p-3 transition-colors {isModified
										? 'border-blue-200 ring-2 ring-blue-500/20'
										: ''}"
								>
									<div class="flex items-center gap-3">
										<div>
											<div class="flex items-center gap-2">
												<span class="text-sm font-medium">{taskName}</span>
												{#if isModified}
													<span class="hidden text-xs font-medium text-blue-600 sm:inline"
														>已修改</span
													>
													<div
														class="h-2 w-2 rounded-full bg-blue-500 sm:hidden"
														title="已修改"
													></div>
												{/if}
											</div>
											<div class="mt-0.5 flex items-center gap-1.5">
												<div class="h-1.5 w-1.5 rounded-full {statusInfo.dotClass}"></div>
												<span class="text-xs {statusInfo.class}">{statusInfo.label}</span>
											</div>
										</div>
									</div>
									<div class="flex gap-1.5">
										{#if isModified}
											<Button
												variant="ghost"
												size="sm"
												onclick={() => resetVideoStatus(index)}
												disabled={loading}
												class="h-7 min-w-[60px] cursor-pointer px-3 text-xs text-gray-600 hover:bg-gray-100"
												title="恢复到原始状态"
											>
												重置
											</Button>
										{/if}
										<Button
											variant={videoStatuses[index] === 0 ? 'default' : 'outline'}
											size="sm"
											onclick={() => setVideoStatus(index, 0)}
											disabled={loading}
											class="h-7 min-w-[60px] cursor-pointer px-3 text-xs {videoStatuses[index] ===
											0
												? 'border-yellow-600 bg-yellow-600 font-medium text-white hover:bg-yellow-700'
												: 'hover:border-yellow-400 hover:bg-yellow-50 hover:text-yellow-700'}"
										>
											未开始
										</Button>
										<Button
											variant={videoStatuses[index] === 7 ? 'default' : 'outline'}
											size="sm"
											onclick={() => setVideoStatus(index, 7)}
											disabled={loading}
											class="h-7 min-w-[60px] cursor-pointer px-3 text-xs {videoStatuses[index] ===
											7
												? 'border-emerald-600 bg-emerald-600 font-medium text-white hover:bg-emerald-700'
												: 'hover:border-emerald-400 hover:bg-emerald-50 hover:text-emerald-700'}"
										>
											已完成
										</Button>
									</div>
								</div>
							{/each}
						</div>
					</div>
				</div>

				<!-- 分页状态编辑 -->
				<div>
					<h3 class="mb-4 text-base font-medium">分页状态</h3>
					<div class="bg-card rounded-lg border p-4">
						<div class="space-y-3">
							{#each pageTaskNames as taskName, index (index)}
								{@const statusInfo = getStatusInfo(pageStatuses[index])}
								{@const isModified = pageStatuses[index] !== null}
								<div
									class="bg-background hover:bg-muted/30 flex items-center justify-between rounded-md border p-3 transition-colors {isModified
										? 'border-blue-200 ring-2 ring-blue-500/20'
										: ''}"
								>
									<div class="flex items-center gap-3">
										<div>
											<div class="flex items-center gap-2">
												<span class="text-sm font-medium">{taskName}</span>
												{#if isModified}
													<span class="hidden text-xs font-medium text-blue-600 sm:inline"
														>已修改</span
													>
													<div
														class="h-2 w-2 rounded-full bg-blue-500 sm:hidden"
														title="已修改"
													></div>
												{/if}
											</div>
											<div class="mt-0.5 flex items-center gap-1.5">
												<div class="h-1.5 w-1.5 rounded-full {statusInfo.dotClass}"></div>
												<span class="text-xs {statusInfo.class}">{statusInfo.label}</span>
											</div>
										</div>
									</div>
									<div class="flex gap-1.5">
										{#if isModified}
											<Button
												variant="ghost"
												size="sm"
												onclick={() => resetPageStatus(index)}
												disabled={loading}
												class="h-7 min-w-[60px] cursor-pointer px-3 text-xs text-gray-600 hover:bg-gray-100"
												title="恢复到原始状态"
											>
												重置
											</Button>
										{/if}
										<Button
											variant={pageStatuses[index] === 0 ? 'default' : 'outline'}
											size="sm"
											onclick={() => setPageStatus(index, 0)}
											disabled={loading}
											class="h-7 min-w-[60px] cursor-pointer px-3 text-xs {pageStatuses[index] === 0
												? 'border-yellow-600 bg-yellow-600 font-medium text-white hover:bg-yellow-700'
												: 'hover:border-yellow-400 hover:bg-yellow-50 hover:text-yellow-700'}"
										>
											未开始
										</Button>
										<Button
											variant={pageStatuses[index] === 7 ? 'default' : 'outline'}
											size="sm"
											onclick={() => setPageStatus(index, 7)}
											disabled={loading}
											class="h-7 min-w-[60px] cursor-pointer px-3 text-xs {pageStatuses[index] === 7
												? 'border-emerald-600 bg-emerald-600 font-medium text-white hover:bg-emerald-700'
												: 'hover:border-emerald-400 hover:bg-emerald-50 hover:text-emerald-700'}"
										>
											已完成
										</Button>
									</div>
								</div>
							{/each}
						</div>
					</div>
				</div>
			</div>
		</div>

		<SheetFooter class="bg-background flex gap-2 border-t px-6 pt-4">
			<Button
				variant="outline"
				onclick={resetAllStatuses}
				disabled={!hasAnyChanges || loading}
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
