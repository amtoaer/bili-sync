<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import SubscriptionCard from '$lib/components/subscription-card.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import type { CollectionWithSubscriptionStatus, ApiError } from '$lib/types';

	let collections: CollectionWithSubscriptionStatus[] = [];
	let totalCount = 0;
	let currentPage = 0;
	let loading = false;

	const pageSize = 50;

	async function loadCollections(page: number = 0) {
		loading = true;
		try {
			const response = await api.getFollowedCollections(page + 1, pageSize); // API使用1基索引
			collections = response.data.collections;
			totalCount = response.data.total;
		} catch (error) {
			console.error('加载合集失败:', error);
			toast.error('加载合集失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function handleSubscriptionSuccess() {
		// 重新加载数据以获取最新状态
		loadCollections(currentPage);
	}

	async function handlePageChange(page: number) {
		currentPage = page;
		await loadCollections(page);
	}

	onMount(async () => {
		setBreadcrumb([
			{
				label: '我关注的合集'
			}
		]);
		await loadCollections();
	});

	$: totalPages = Math.ceil(totalCount / pageSize);
</script>

<svelte:head>
	<title>关注的合集 - Bili Sync</title>
</svelte:head>

<div>
	<div class="mb-6 flex items-center justify-between">
		<div class="text-sm font-medium">
			{#if !loading}
				共 {totalCount} 个合集
			{/if}
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if collections.length > 0}
		<div
			style="display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 16px; width: 100%; max-width: none; justify-items: start;"
		>
			{#each collections as collection (collection.sid)}
				<div style="max-width: 450px; width: 100%;">
					<SubscriptionCard
						item={collection}
						type="collection"
						onSubscriptionSuccess={handleSubscriptionSuccess}
					/>
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
				<p class="text-muted-foreground">暂无合集数据</p>
				<p class="text-muted-foreground text-sm">请先在B站关注一些合集，或检查账号配置</p>
			</div>
		</div>
	{/if}
</div>
