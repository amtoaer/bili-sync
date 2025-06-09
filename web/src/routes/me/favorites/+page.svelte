<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import SubscriptionCard from '$lib/components/subscription-card.svelte';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { appStateStore, ToQuery } from '$lib/stores/filter';
	import api from '$lib/api';
	import type { FavoriteWithSubscriptionStatus, ApiError } from '$lib/types';

	let favorites: FavoriteWithSubscriptionStatus[] = [];
	let loading = false;

	async function loadFavorites() {
		loading = true;
		try {
			const response = await api.getCreatedFavorites();
			favorites = response.data.favorites;
		} catch (error) {
			console.error('加载收藏夹失败:', error);
			toast.error('加载收藏夹失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function handleSubscriptionSuccess() {
		// 重新加载数据以获取最新状态
		loadFavorites();
	}

	onMount(async () => {
		setBreadcrumb([
			{
				label: '主页',
				onClick: () => {
					goto(`/${ToQuery($appStateStore)}`);
				}
			},
			{ label: '我的收藏夹', isActive: true }
		]);

		await loadFavorites();
	});
</script>

<svelte:head>
	<title>我的收藏夹 - Bili Sync</title>
</svelte:head>

<div>
	<div class="mb-6 flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold">我的收藏夹</h1>
			<p class="text-muted-foreground mt-1">管理您在B站创建的收藏夹订阅</p>
		</div>
		<div class="text-muted-foreground text-sm">
			{#if !loading}
				共 {favorites.length} 个收藏夹
			{/if}
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if favorites.length > 0}
		<div
			style="display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 16px; width: 100%; max-width: none; justify-items: start;"
		>
			{#each favorites as favorite (favorite.fid)}
				<div style="max-width: 450px; width: 100%;">
					<SubscriptionCard
						item={favorite}
						type="favorite"
						onSubscriptionSuccess={handleSubscriptionSuccess}
					/>
				</div>
			{/each}
		</div>
	{:else}
		<div class="flex items-center justify-center py-12">
			<div class="space-y-2 text-center">
				<p class="text-muted-foreground">暂无收藏夹数据</p>
				<p class="text-muted-foreground text-sm">请先在B站创建收藏夹，或检查账号配置</p>
			</div>
		</div>
	{/if}
</div>
