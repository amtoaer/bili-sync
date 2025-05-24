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
	const videoStatusLabels = ['视频封面', '视频信息', 'Up主头像', 'Up主信息', '分P下载'];
	const pageStatusLabels = ['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕'];

	// 更详细的状态描述
	const statusDescriptions = [
		'未下载', 
		'等待下载',
		'下载中', 
		'已下载', 
		'下载失败',
		'重试中',
		'暂停',
		'完成'
	];

	// 状态对应的颜色样式
	const statusColorClasses = {
		'未下载': 'bg-gray-100 text-gray-800',
		'等待下载': 'bg-yellow-100 text-yellow-800',
		'下载中': 'bg-blue-100 text-blue-800',
		'已下载': 'bg-green-100 text-green-800',
		'下载失败': 'bg-red-100 text-red-800',
		'重试中': 'bg-orange-100 text-orange-800',
		'暂停': 'bg-purple-100 text-purple-800',
		'完成': 'bg-emerald-100 text-emerald-800'
	};

	let prevCollapseSignal = collapseSignal;
	$: if (collapseSignal !== prevCollapseSignal) {
		showDetail = false;
		prevCollapseSignal = collapseSignal;
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
			<div class="flex space-x-1 flex-wrap">
				{#each video.download_status as status, i}
					<Badge variant="outline" class={status >= 0 && status < 8 ? statusColorClasses[statusDescriptions[status]] : 'bg-gray-100'}>
						{videoStatusLabels[i]}: {status >= 0 && status < 8 ? statusDescriptions[status] : `未知状态(${status})`}
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
							<div class="flex space-x-1 flex-wrap">
								{#each page.download_status as status, i}
									<Badge variant="outline" class={status >= 0 && status < 8 ? statusColorClasses[statusDescriptions[status]] : 'bg-gray-100'}>
										{pageStatusLabels[i]}: {status >= 0 && status < 8 ? statusDescriptions[status] : `未知状态(${status})`}
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
