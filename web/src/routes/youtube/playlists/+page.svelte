<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import SearchBar from '$lib/components/search-bar.svelte';
	import YoutubePlaylistCard from '$lib/components/youtube-playlist-card.svelte';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import type { ApiError, YoutubePlaylist } from '$lib/types';

	let playlists: YoutubePlaylist[] = [];
	let loading = false;
	let searchQuery = '';

	async function loadPlaylists() {
		loading = true;
		try {
			const response = await api.getYoutubePlaylists();
			playlists = response.data.playlists;
		} catch (error) {
			console.error('加载 YouTube 播放列表失败：', error);
			toast.error('加载 YouTube 播放列表失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function handleAddSuccess() {
		loadPlaylists();
	}

	function handleSearch(query: string) {
		searchQuery = query;
	}

	$: filteredPlaylists = playlists.filter((playlist) => {
		const keyword = searchQuery.toLowerCase();
		return (
			playlist.name.toLowerCase().includes(keyword) ||
			(playlist.ownerName || '').toLowerCase().includes(keyword)
		);
	});

	onMount(() => {
		setBreadcrumb([{ label: 'YouTube 我的播放列表' }]);
		loadPlaylists();
	});
</script>

<svelte:head>
	<title>YouTube 我的播放列表 - Bili Sync</title>
</svelte:head>

<div>
	<div class="mb-4 flex items-center justify-between">
		<SearchBar
			placeholder="搜索 YouTube 播放列表.."
			value={searchQuery}
			onSearch={handleSearch}
		></SearchBar>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if filteredPlaylists.length > 0}
		<div
			style="display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 16px; width: 100%; max-width: none; justify-items: start;"
		>
			{#each filteredPlaylists as playlist (playlist.playlistId)}
				<div style="max-width: 450px; width: 100%;">
					<YoutubePlaylistCard item={playlist} onAddSuccess={handleAddSuccess} />
				</div>
			{/each}
		</div>
	{:else}
		<div class="flex items-center justify-center py-12">
			<div class="space-y-2 text-center">
				<p class="text-muted-foreground">暂无 YouTube 播放列表数据</p>
				<p class="text-muted-foreground text-sm">请先在设置页粘贴并保存有效的 YouTube Cookie</p>
			</div>
		</div>
	{/if}
</div>
