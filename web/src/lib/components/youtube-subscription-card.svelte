<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import YoutubeSubscriptionDialog from './youtube-subscription-dialog.svelte';
	import { CheckIcon, PlusIcon, VideoIcon } from '@lucide/svelte/icons';
	import type { YoutubeSubscription } from '$lib/types';

	export let item: YoutubeSubscription;
	export let onSubscriptionSuccess: (() => void) | null = null;

	let dialogOpen = false;

	function handleSubscriptionSuccess() {
		item.subscribed = true;
		onSubscriptionSuccess?.();
	}
</script>

<Card class="border-border/50 group flex h-full flex-col transition-all hover:shadow-lg">
	<CardHeader class="shrink-0">
		<div class="flex items-start gap-3">
			<div class="bg-accent/50 flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-full">
				{#if item.thumbnail}
					<img src={item.thumbnail} alt={item.name} class="h-full w-full object-cover" loading="lazy" />
				{:else}
					<VideoIcon class="text-muted-foreground h-5 w-5" />
				{/if}
			</div>
			<div class="min-w-0 flex-1 space-y-2">
				<div class="flex items-start justify-between gap-2">
					<CardTitle class="line-clamp-2 text-sm leading-relaxed font-medium">
						{item.name}
					</CardTitle>
					<Badge variant="secondary" class="shrink-0 text-xs">
						{item.subscribed ? '已添加' : 'YouTube'}
					</Badge>
				</div>
				<p class="text-muted-foreground line-clamp-2 break-all text-xs">{item.url}</p>
			</div>
		</div>
	</CardHeader>

	<CardContent class="flex flex-1 items-end justify-end">
		{#if item.subscribed}
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

<YoutubeSubscriptionDialog bind:open={dialogOpen} {item} onSuccess={handleSubscriptionSuccess} />
