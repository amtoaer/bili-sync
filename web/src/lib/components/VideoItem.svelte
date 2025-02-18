<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { getVideo, resetVideo } from '$lib/api';
	import { toast } from 'svelte-sonner';
	import type { VideoResponse, VideoInfo, ResetVideoResponse } from '$lib/types';

	export let video: VideoInfo;
	export let collapseSignal: boolean = false;

	let showDetail = false;
	let detail: VideoResponse | null = null;
	let loading = false;

	// 定义视频和页面各状态的名称映射
	const videoStatusLabels = ['视频封面', '视频信息', 'Up 主头像', 'Up 主信息', '分 P 下载'];
	const pageStatusLabels = ['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕'];

	let prevCollapseSignal = collapseSignal;
	$: if (collapseSignal !== prevCollapseSignal) {
		showDetail = false;
		prevCollapseSignal = collapseSignal;
	}

	function getVariant(status: number): 'warning' | 'success' | 'destructive' {
		if (status === 0) return 'warning';
		if (status === 7) return 'success';
		return 'destructive';
	}

	async function toggleDetail() {
		showDetail = !showDetail;
		if (showDetail && (!detail || detail.video.id !== video.id)) {
			loading = true;
			detail = await getVideo(video.id);
			loading = false;
		}
	}

	// 修改重置函数：调用 resetVideo 后重新获取视频详情
	async function resetVideoItem() {
		loading = true;
		try {
			const res: ResetVideoResponse = await resetVideo(video.id);
			// 重置后重新加载视频详情，并更新视频信息
			const newDetail = await getVideo(video.id);
			detail = newDetail;
			video = newDetail.video;
			// 根据返回的 resetted 显示提示
			if (res.resetted) {
				toast.success('重置成功', {
					description: `已重置视频与视频的 ${res.pages.length} 条 page.`
				});
			} else {
				toast.info('重置无效', {
					description: '所有任务均成功，无需重置'
				});
			}
		} catch (error) {
			console.error(error);
			toast.error('重置失败', { description: `错误信息：${error}` });
		}
		loading = false;
	}
</script>

<div class="my-2 rounded border p-4">
	<div class="flex items-center justify-between">
		<div>
			<h3>{video.name}</h3>
			<div class="flex space-x-1">
				{#each video.download_status as status, i}
					<Badge variant={getVariant(status)}>
						{videoStatusLabels[i]}: {status === 0
							? '未开始'
							: status === 7
								? '已完成'
								: `失败 ${status} 次`}
					</Badge>
				{/each}
			</div>
			<p class="text-sm text-gray-500">{video.upper_name}</p>
		</div>
		<div class="flex space-x-2">
			<Button onclick={toggleDetail}>
				{showDetail ? '收起' : '展开'}
			</Button>
			<Button onclick={resetVideoItem}>重置</Button>
		</div>
	</div>
	{#if showDetail}
		{#if loading}
			<p>加载详情...</p>
		{:else if detail}
			<div class="mt-2">
				<h4 class="font-semibold">视频详情</h4>
				<div>
					{#each detail.pages as page}
						<div class="border-t py-1">
							<p>ID: {page.id} - 名称: {page.name}</p>
							<div class="flex space-x-1">
								{#each page.download_status as status, i}
									<Badge variant={getVariant(status)}>
										{pageStatusLabels[i]}: {status === 0
											? '未开始'
											: status === 7
												? '已完成'
												: `失败 ${status} 次`}
									</Badge>
								{/each}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
