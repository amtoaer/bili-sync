<script context="module">
	// 类型定义
	export interface VideoListModelItem {
		id: number;
		name: string;
	}
	export interface VideoListModel {
		collection: VideoListModelItem[];
		favorite: VideoListModelItem[];
		submission: VideoListModelItem[];
		watch_later: VideoListModelItem[];
	}
</script>

<script lang="ts">
	// 接收参数
	export let listModel: VideoListModel | null = null;
	export let sidebarOpen: boolean;
	export let toggleSidebar: () => void;
	export let selectFilter: (key: keyof VideoListModel, id: number) => void;
	export let clearFilter: () => void;
</script>

<div class="p-4">
	<!-- 标题栏和折叠控制按钮 -->
	<div class="flex justify-between items-center mb-4">
		<h2 class="text-xl font-bold">视频筛选</h2>
		<button class="btn btn-sm btn-outline" on:click={toggleSidebar}>
			{sidebarOpen ? '收起' : '展开'}
		</button>
	</div>
	{#if sidebarOpen}
		<div class="overflow-y-auto">
			<ul class="menu bg-base-200 rounded-box shadow-md p-2">
				{#if listModel}
					{#each Object.entries(listModel) as [key, items]}
						<li class="menu-title uppercase text-xs mt-4">{key}</li>
						{#each items as item}
							<li>
								<button
									class="normal-case hover:text-primary"
									on:click={() => selectFilter(key as keyof VideoListModel, item.id)}
								>
									{item.name}
								</button>
							</li>
						{/each}
					{/each}
					<li class="mt-4">
						<button class="btn btn-ghost btn-xs" on:click={clearFilter}>清除筛选</button>
					</li>
				{:else}
					<li class="text-xs opacity-70">加载中...</li>
				{/if}
			</ul>
		</div>
	{/if}
</div>
