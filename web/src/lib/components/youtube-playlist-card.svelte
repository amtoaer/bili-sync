<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import YoutubePlaylistDialog from './youtube-playlist-dialog.svelte';
	import { CheckIcon, ListVideoIcon, PlusIcon } from '@lucide/svelte/icons';
	import type { YoutubePlaylist } from '$lib/types';

	export let item: YoutubePlaylist;
	export let onAddSuccess: (() => void) | null = null;

	let dialogOpen = false;

	function handleAddSuccess() {
		item.added = true;
		onAddSuccess?.();
	}
</script>

<Card class="border-border/50 group flex h-full flex-col transition-all hover:shadow-lg">
	<CardHeader class="shrink-0">
		<div class="flex items-start gap-3">
			<div class="bg-accent/50 flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-xl">
				{#if item.thumbnail}
					<img src={item.thumbnail} alt={item.name} class="h-full w-full object-cover" loading="lazy" />
				{:else}
					<ListVideoIcon class="text-muted-foreground h-5 w-5" />
				{/if}
			</div>
			<div class="min-w-0 flex-1 space-y-2">
				<div class="flex items-start justify-between gap-2">
					<CardTitle class="line-clamp-2 text-sm leading-relaxed font-medium">
						{item.name}
					</CardTitle>
					<Badge variant="secondary" class="shrink-0 text-xs">
						{item.added ? '已添加' : '播放列表'}
					</Badge>
				</div>
				{#if item.ownerName}
					<p class="text-muted-foreground line-clamp-1 text-xs">创建者：{item.ownerName}</p>
				{/if}
				{#if item.videoCount}
					<p class="text-muted-foreground line-clamp-1 text-xs">视频数：{item.videoCount}</p>
				{/if}
				<p class="text-muted-foreground line-clamp-2 break-all text-xs">{item.url}</p>
			</div>
		</div>
	</CardHeader>

	<CardContent class="flex flex-1 items-end justify-end">
		{#if item.added}
			<Button size="sm" variant="outline" disabled class="h-8 text-xs">
				<CheckIcon class="mr-1 h-3 w-3" />
				已添加
			</Button>
		{:else}
			<Button size="sm" variant="outline" onclick={() => (dialogOpen = true)} class="h-8 text-xs">
				<PlusIcon class="mr-1 h-3 w-3" />
				添加到视频源
			</Button>
		{/if}
	</CardContent>
</Card>

<YoutubePlaylistDialog bind:open={dialogOpen} {item} onSuccess={handleAddSuccess} />
