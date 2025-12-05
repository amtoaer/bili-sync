<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import SubscriptionCard from '$lib/components/subscription-card.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import type { Followed, ApiError } from '$lib/types';

	let uppers: Followed[] = [];
	let totalCount = 0;
	let currentPage = 0;
	let loading = false;

	const pageSize = 50;

	async function loadUppers(page: number = 0) {
		loading = true;
		try {
			const response = await api.getFollowedUppers(page + 1, pageSize); // API 使用 1 基索引
			uppers = response.data.uppers;
			totalCount = response.data.total;
		} catch (error) {
			console.error('加载 UP 主失败：', error);
			toast.error('加载 UP 主失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function handleSubscriptionSuccess() {
		// 重新加载数据以获取最新状态
		loadUppers(currentPage);
	}

	async function handlePageChange(page: number) {
		currentPage = page;
		await loadUppers(page);
	}

	onMount(async () => {
		setBreadcrumb([{ label: '我关注的 UP 主' }]);
		await loadUppers();
	});

	$: totalPages = Math.ceil(totalCount / pageSize);
</script>

<svelte:head>
	<title>我关注的 UP 主 - Bili Sync</title>
</svelte:head>

<div>
	<div class="mb-6 flex items-center justify-between">
		<div class="flex items-center gap-6">
			{#if !loading}
				<div class=" text-sm font-medium">
					共 {totalCount} 个 UP 主
				</div>
				<div class=" text-sm font-medium">
					当前第 {currentPage + 1} / {totalPages} 页
				</div>
			{/if}
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if uppers.length > 0}
		<div
			style="display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 16px; width: 100%; max-width: none; justify-items: start;"
		>
			{#each uppers as upper (upper.mid)}
				<div style="max-width: 450px; width: 100%;">
					<SubscriptionCard item={upper} onSubscriptionSuccess={handleSubscriptionSuccess} />
				</div>
			{/each}
		</div>

		<!-- 分页组件 -->
		{#if totalPages > 1}
			<Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
		{/if}
	{:else}
		<div class="flex items-center justify-center py-12">
			<div class="space-y-2 text-center">
				<p class="text-muted-foreground">暂无 UP 主数据</p>
				<p class="text-muted-foreground text-sm">请先在 B 站关注一些 UP 主，或检查账号配置</p>
			</div>
		</div>
	{/if}
</div>
