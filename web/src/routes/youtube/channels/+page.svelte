<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import SearchBar from '$lib/components/search-bar.svelte';
	import YoutubeSubscriptionCard from '$lib/components/youtube-subscription-card.svelte';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import type { ApiError, YoutubeSubscription } from '$lib/types';

	let channels: YoutubeSubscription[] = [];
	let loading = false;
	let searchQuery = '';

	async function loadChannels() {
		loading = true;
		try {
			const response = await api.getYoutubeChannels();
			channels = response.data.channels;
		} catch (error) {
			console.error('加载 YouTube 订阅频道失败：', error);
			toast.error('加载 YouTube 订阅频道失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function handleSubscriptionSuccess() {
		loadChannels();
	}

	function handleSearch(query: string) {
		searchQuery = query;
	}

	$: filteredChannels = channels.filter((channel) =>
		channel.name.toLowerCase().includes(searchQuery.toLowerCase())
	);

	onMount(() => {
		setBreadcrumb([{ label: 'YouTube 订阅频道' }]);
		loadChannels();
	});
</script>

<svelte:head>
	<title>YouTube 订阅频道 - Bili Sync</title>
</svelte:head>

<div>
	<div class="mb-4 flex items-center justify-between">
		<SearchBar
			placeholder="搜索 YouTube 频道.."
			value={searchQuery}
			onSearch={handleSearch}
		></SearchBar>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if filteredChannels.length > 0}
		<div
			style="display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 16px; width: 100%; max-width: none; justify-items: start;"
		>
			{#each filteredChannels as channel (channel.channelId)}
				<div style="max-width: 450px; width: 100%;">
					<YoutubeSubscriptionCard item={channel} onSubscriptionSuccess={handleSubscriptionSuccess} />
				</div>
			{/each}
		</div>
	{:else}
		<div class="flex items-center justify-center py-12">
			<div class="space-y-2 text-center">
				<p class="text-muted-foreground">暂无 YouTube 频道数据</p>
				<p class="text-muted-foreground text-sm">请先在设置页粘贴并保存 YouTube Cookie</p>
			</div>
		</div>
	{/if}
</div>
