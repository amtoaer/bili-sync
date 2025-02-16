<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';

	import { getVideo } from '$lib/api';
	import type { VideoDetail } from '$lib/types';
	export let video: { id: number; name: string; upper_name: string; download_status: number[] };

	let showDetail = false;
	let detail: VideoDetail | null = null;
	let loading = false;

	async function toggleDetail() {
		showDetail = !showDetail;
		if (showDetail && (!detail || detail.video.id !== video.id)) {
			loading = true;
			detail = await getVideo(video.id);
			loading = false;
		}
	}
</script>

<div class="my-2 rounded border p-4">
	<div class="flex items-center justify-between">
		<div>
			<h3>{video.name}</h3>
			<p class="text-sm text-gray-500">{video.upper_name}</p>
		</div>
		<Button onclick={toggleDetail}>
			{showDetail ? '收起' : '展开'}
		</Button>
	</div>
	{#if showDetail}
		<!-- 展示详情内容 -->
		{#if loading}
			<p>加载详情...</p>
		{:else if detail}
			<div class="mt-2">
				<h4 class="font-semibold">视频详情</h4>
				<div>
					<!-- 展示视频的各个 page 信息 -->
					{#each detail.pages as page}
						<div class="border-t py-1">
							<p>ID: {page.id} - 名称: {page.name}</p>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
