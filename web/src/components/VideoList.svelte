<script context="module">
	export interface VideoInfo {
		id: number;
		name: string;
		upper_name: string;
		download_status: number[];
	}
</script>

<script lang="ts">
	export let videos: VideoInfo[] = [];
	export let loadingVideos: boolean = false;
	export let gridView: boolean = true;
	export let setGridView: (value: boolean) => void;

	// 辅助函数：将状态数字转换为文字描述
	function getStatusLabel(s: number): string {
		if (s === 0) return '待执行';
		else if (s === 7) return '完成';
		else return `重试 ${s} 次`;
	}
	// 根据状态返回 daisyUI badge 样式
	function getBadgeClass(s: number): string {
		if (s === 0) return 'badge badge-info';
		else if (s === 7) return 'badge badge-success';
		else return 'badge badge-warning';
	}
</script>

<div class="p-4">
	<div class="flex items-center justify-between mb-4">
		<h1 class="text-2xl font-bold">视频列表</h1>
		<div class="btn-group">
			<!-- 当前选中按钮使用 btn-active 辅助样式 -->
			<button class="btn btn-sm" class:btn-active={gridView} on:click={() => setGridView(true)}>
				网格
			</button>
			<button class="btn btn-sm" class:btn-active={!gridView} on:click={() => setGridView(false)}>
				列表
			</button>
		</div>
	</div>

	{#if loadingVideos}
		<div class="alert alert-info shadow-md">
			<div>
				<span>加载中...</span>
			</div>
		</div>
	{:else if videos.length === 0}
		<div class="alert alert-warning shadow-md">
			<div>
				<span>没有找到视频</span>
			</div>
		</div>
	{:else if gridView}
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each videos as video (video.id)}
				<div class="card bg-base-100 shadow-lg hover:shadow-2xl transition">
					<div class="card-body">
						<h2 class="card-title text-lg">{video.name}</h2>
						<p class="text-sm text-gray-500">{video.upper_name}</p>
						<div class="flex flex-wrap gap-2 mt-2">
							{#each video.download_status as s, idx}
								<div class={getBadgeClass(s)}>
									<span class="text-xs">子任务 {idx + 1}:</span>
									<span class="ml-1 text-xs">{getStatusLabel(s)}</span>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<ul class="space-y-4">
			{#each videos as video (video.id)}
				<li class="card bg-base-100 shadow-lg p-4 hover:shadow-2xl transition">
					<h2 class="card-title text-lg">{video.name}</h2>
					<p class="text-sm text-gray-500">{video.upper_name}</p>
					<div class="flex flex-wrap gap-2 mt-2">
						{#each video.download_status as s, idx}
							<div class={getBadgeClass(s)}>
								<span class="text-xs">子任务 {idx + 1}:</span>
								<span class="ml-1 text-xs">{getStatusLabel(s)}</span>
							</div>
						{/each}
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</div>
