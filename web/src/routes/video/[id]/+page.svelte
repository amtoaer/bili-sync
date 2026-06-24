<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import api from '$lib/api';
	import SquareArrowOutUpRightIcon from '@lucide/svelte/icons/square-arrow-out-up-right';
	import type { ApiError, VideoResponse, UpdateVideoStatusRequest } from '$lib/types';
	import {
		RotateCcwIcon,
		SquarePenIcon,
		BrushCleaningIcon,
		RefreshCwIcon
	} from '@lucide/svelte/icons';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { appStateStore, ToQuery } from '$lib/stores/filter';
	import VideoCard from '$lib/components/video-card.svelte';
	import StatusEditor from '$lib/components/status-editor.svelte';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { DANMAKU_GENERATION_LABELS, formatRelativeTime } from '$lib/consts';
	import { toast } from 'svelte-sonner';

	let videoData: VideoResponse | null = null;
	let loading = false;
	let error: string | null = null;
	let resetDialogOpen = false;
	let resetting = false;
	let clearAndResetDialogOpen = false;
	let clearAndResetting = false;
	let statusEditorOpen = false;
	let statusEditorLoading = false;
	let refreshingDanmaku = false;
	let refreshingPageDanmaku = new Set<number>();

	async function loadVideoDetail() {
		const videoId = parseInt($page.params.id!);
		if (isNaN(videoId)) {
			error = '无效的视频 ID';
			toast.error('无效的视频 ID');
			return;
		}
		loading = true;
		error = null;
		try {
			const result = await api.getVideo(videoId);
			videoData = result.data;
		} catch (error) {
			console.error('加载视频详情失败：', error);
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
				label: '视频',
				href: `/${ToQuery($appStateStore)}`
			},
			{ label: '视频详情' }
		]);
	});

	// 监听路由参数变化
	$: if ($page.params.id) {
		loadVideoDetail();
	}

	async function handleStatusEditorSubmit(request: UpdateVideoStatusRequest) {
		if (!videoData) return;

		statusEditorLoading = true;
		try {
			const result = await api.updateVideoStatus(videoData.video.id, request);
			const data = result.data;

			if (data.success) {
				// 更新本地数据
				videoData = {
					video: data.video,
					pages: data.pages
				};
				statusEditorOpen = false;
				toast.success('状态更新成功');
			} else {
				toast.error('状态更新失败');
			}
		} catch (error) {
			console.error('状态更新失败：', error);
			toast.error('状态更新失败', {
				description: (error as ApiError).message
			});
		} finally {
			statusEditorLoading = false;
		}
	}

	async function handleReset(forceReset: boolean) {
		if (!videoData) return;
		try {
			const result = await api.resetVideoStatus(videoData.video.id, { force: forceReset });
			const data = result.data;
			if (data.resetted) {
				videoData = {
					video: data.video,
					pages: data.pages
				};
				toast.success('重置成功');
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

	async function handleRefreshDanmaku() {
		if (!videoData || refreshingDanmaku) return;
		refreshingDanmaku = true;
		try {
			const result = await api.refreshDanmakuForVideo(videoData.video.id);
			toast.success('弹幕刷新完成', {
				description: `已成功刷新 ${result.data.refreshed} 个分页`
			});
			await loadVideoDetail();
		} catch (error) {
			console.error('弹幕刷新失败:', error);
			toast.error('弹幕刷新失败', {
				description: (error as ApiError).message
			});
		} finally {
			refreshingDanmaku = false;
		}
	}

	async function handleRefreshPageDanmaku(pageId: number) {
		if (refreshingPageDanmaku.has(pageId)) return;
		refreshingPageDanmaku = new Set([...refreshingPageDanmaku, pageId]);
		try {
			await api.refreshDanmakuForPage(pageId);
			toast.success('弹幕刷新完成');
			await loadVideoDetail();
		} catch (error) {
			console.error('弹幕刷新失败:', error);
			toast.error('弹幕刷新失败', {
				description: (error as ApiError).message
			});
		} finally {
			const next = new Set(refreshingPageDanmaku);
			next.delete(pageId);
			refreshingPageDanmaku = next;
		}
	}

	async function handleClearAndReset() {
		if (!videoData) return;
		try {
			const result = await api.clearAndResetVideoStatus(videoData.video.id);
			const data = result.data;
			videoData = {
				video: data.video,
				pages: []
			};
			if (data.warning) {
				toast.warning('清空重置成功', {
					description: data.warning
				});
			} else {
				toast.success('清空重置成功', {
					description: `视频「${data.video.name}」已清空重置`
				});
			}
		} catch (error) {
			console.error('清空重置失败：', error);
			toast.error('清空重置失败', {
				description: (error as ApiError).message
			});
		}
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
			<div class="flex gap-2">
				<Button
					size="sm"
					variant="outline"
					class="shrink-0 cursor-pointer "
					onclick={() => (statusEditorOpen = true)}
					disabled={statusEditorLoading}
				>
					<SquarePenIcon class="mr-2 h-4 w-4" />
					编辑状态
				</Button>
				<Button
					size="sm"
					variant="outline"
					class="shrink-0 cursor-pointer "
					onclick={() => (resetDialogOpen = true)}
					disabled={resetting || clearAndResetting}
				>
					<RotateCcwIcon class="mr-2 h-4 w-4 {resetting ? 'animate-spin' : ''}" />
					重置
				</Button>
				<Button
					size="sm"
					variant="outline"
					class="shrink-0 cursor-pointer "
					onclick={() => (clearAndResetDialogOpen = true)}
					disabled={resetting || clearAndResetting}
				>
					<BrushCleaningIcon class="mr-2 h-4 w-4 {clearAndResetting ? 'animate-spin' : ''}" />
					清空重置
				</Button>
				<Button
					size="sm"
					variant="outline"
					class="shrink-0 cursor-pointer "
					onclick={handleRefreshDanmaku}
					disabled={refreshingDanmaku || resetting || clearAndResetting}
					title="立即重新拉取所有分页的弹幕（忽略更新策略）"
				>
					<RefreshCwIcon class="mr-2 h-4 w-4 {refreshingDanmaku ? 'animate-spin' : ''}" />
					刷新弹幕
				</Button>
				<Button
					size="sm"
					variant="outline"
					class="shrink-0 cursor-pointer "
					onclick={() =>
						window.open(`https://www.bilibili.com/video/${videoData?.video.bvid}/`, '_blank')}
					disabled={statusEditorLoading}
				>
					<SquareArrowOutUpRightIcon class="mr-2 h-4 w-4" />
					在 B 站打开
				</Button>
			</div>
		</div>

		<div style="margin-bottom: 1rem;">
			<VideoCard
				video={videoData.video}
				mode="detail"
				showActions={false}
				taskNames={['视频封面', '视频信息', 'UP 主头像', 'UP 主信息', '分页下载']}
				bind:resetDialogOpen
				bind:resetting
				bind:clearAndResetDialogOpen
				bind:clearAndResetting
				onReset={handleReset}
				onClearAndReset={handleClearAndReset}
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
						<div class="space-y-2">
							<VideoCard
								video={{
									id: pageInfo.id,
									name: `P${pageInfo.pid}: ${pageInfo.name}`,
									upper_name: '',
									download_status: pageInfo.download_status,
									should_download: videoData.video.should_download,
									valid: videoData.video.valid
								}}
								mode="page"
								showActions={false}
								customTitle="P{pageInfo.pid}: {pageInfo.name}"
								customSubtitle=""
								taskNames={['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕']}
							/>
							<div
								class="text-muted-foreground flex items-center justify-between gap-2 px-2 text-xs"
							>
								<div class="flex items-center gap-2">
									{#if pageInfo.danmaku_sync_generation > 0}
										{@const label = DANMAKU_GENERATION_LABELS[pageInfo.danmaku_sync_generation]}
										<Badge variant={label?.variant ?? 'outline'} class="text-[10px]">
											弹幕 · {label?.text ?? '—'}
										</Badge>
									{/if}
									<span title={pageInfo.danmaku_last_synced_at ?? ''}>
										{formatRelativeTime(pageInfo.danmaku_last_synced_at)}
									</span>
								</div>
								<Button
									size="sm"
									variant="ghost"
									class="h-6 px-2"
									disabled={refreshingPageDanmaku.has(pageInfo.id)}
									onclick={() => handleRefreshPageDanmaku(pageInfo.id)}
									title="仅刷新该分页的弹幕"
								>
									<RefreshCwIcon
										class="mr-1 h-3 w-3 {refreshingPageDanmaku.has(pageInfo.id)
											? 'animate-spin'
											: ''}"
									/>
									刷新
								</Button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{:else}
			<div class="py-12 text-center">
				<div class="space-y-2">
					<p class="text-muted-foreground">暂无分 P 数据</p>
				</div>
			</div>
		{/if}
	</section>

	<!-- 状态编辑器 -->
	{#if videoData}
		<StatusEditor
			bind:open={statusEditorOpen}
			video={videoData.video}
			pages={videoData.pages}
			loading={statusEditorLoading}
			onsubmit={handleStatusEditorSubmit}
		/>
	{/if}
{/if}
