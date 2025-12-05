<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	import SubscriptionCard from '$lib/components/subscription-card.svelte';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';

	import api from '$lib/api';
	import type { Followed, ApiError } from '$lib/types';
	import { getFollowedKey } from '$lib/utils';

	let favorites: Followed[] = [];
	let loading = false;

	async function loadFavorites() {
		loading = true;
		try {
			const response = await api.getCreatedFavorites();
			favorites = response.data.favorites;
		} catch (error) {
			console.error('加载收藏夹失败：', error);
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
		setBreadcrumb([{ label: '我创建的收藏夹' }]);

		await loadFavorites();
	});
</script>

<svelte:head>
	<title>我创建的收藏夹 - Bili Sync</title>
</svelte:head>

<div>
	<div class="mb-6 flex items-center justify-between">
		<div class="text-sm font-medium">
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
			{#each favorites as favorite (getFollowedKey(favorite))}
				<div style="max-width: 450px; width: 100%;">
					<SubscriptionCard item={favorite} onSubscriptionSuccess={handleSubscriptionSuccess} />
				</div>
			{/each}
		</div>
	{:else}
		<div class="flex items-center justify-center py-12">
			<div class="space-y-2 text-center">
				<p class="text-muted-foreground">暂无收藏夹数据</p>
				<p class="text-muted-foreground text-sm">请先在 B 站创建收藏夹，或检查账号配置</p>
			</div>
		</div>
	{/if}
</div>
