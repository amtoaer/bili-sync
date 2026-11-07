<script lang="ts">
	import api from '$lib/api';
	import type { ApiError, BangumiPreview } from '$lib/types';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import ImageIcon from '@lucide/svelte/icons/image';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';

	let {
		url,
		active = true,
		wholeSeason = false
	}: {
		url: string;
		active?: boolean;
		wholeSeason?: boolean;
	} = $props();

	let preview = $state<BangumiPreview | null>(null);
	let loading = $state(false);
	let error = $state('');
	let posterFailed = $state(false);
	let requestVersion = 0;
	// The parent uses a shared legacy form object; only changed input should reload metadata.
	const inputValue = $derived(url.trim());

	async function loadPreview(input: string, version: number) {
		try {
			const response = await api.getBangumiPreview(input);
			if (version !== requestVersion || !active || url.trim() !== input) return;
			preview = response.data;
		} catch (cause) {
			if (version !== requestVersion || !active || url.trim() !== input) return;
			error = (cause as ApiError).message || '暂时无法获取番剧信息，请稍后重试。';
		} finally {
			if (version === requestVersion) loading = false;
		}
	}

	$effect(() => {
		const input = inputValue;
		preview = null;
		error = '';
		posterFailed = false;
		loading = active && input.length > 0;
		if (!active || !input) return;
		const version = ++requestVersion;
		const timer = setTimeout(() => void loadPreview(input, version), 500);
		return () => {
			clearTimeout(timer);
			requestVersion++;
		};
	});

	function retry() {
		error = '';
		loading = true;
		void loadPreview(url.trim(), ++requestVersion);
	}

	function formatCount(value: number | null): string {
		if (value === null) return '暂无数据';
		if (value >= 100_000_000) {
			return `${(value / 100_000_000).toLocaleString('zh-CN', { maximumFractionDigits: 1 })}亿`;
		}
		if (value >= 10_000) {
			return `${(value / 10_000).toLocaleString('zh-CN', { maximumFractionDigits: 1 })}万`;
		}
		return value.toLocaleString('zh-CN');
	}

	function episodeTitle(episode: NonNullable<BangumiPreview['episode']>): string {
		const title = /^\d+$/.test(episode.title) ? `第${episode.title}话` : episode.title;
		return [title, episode.longTitle].filter(Boolean).join(' · ');
	}
</script>

{#if active && url.trim()}
	<div aria-live="polite" aria-busy={loading}>
		{#if loading}
			<div
				class="bg-muted/40 text-muted-foreground flex items-center gap-2 rounded-lg border p-4 text-sm"
				role="status"
			>
				<LoaderCircleIcon class="size-4 shrink-0 animate-spin" />
				正在获取番剧信息…
			</div>
		{:else if error}
			<div
				class="border-destructive/30 bg-destructive/5 space-y-2 rounded-lg border p-3"
				role="alert"
			>
				<p class="text-destructive text-sm font-medium">无法获取番剧信息</p>
				<p class="text-muted-foreground text-xs break-words">{error}</p>
				<Button type="button" size="sm" variant="outline" onclick={retry}>
					<RefreshCwIcon class="size-3.5" />重试
				</Button>
			</div>
		{:else if preview}
			<section class="bg-muted/30 space-y-3 rounded-lg border p-3" aria-label="番剧信息预览">
				<div class="flex items-start gap-3">
					<div
						class="bg-muted flex aspect-[3/4] w-20 shrink-0 items-center justify-center overflow-hidden rounded-md sm:w-24"
					>
						{#if preview.cover && !posterFailed}
							<img
								src={preview.cover}
								alt={`${preview.title}海报`}
								referrerpolicy="no-referrer"
								class="h-full w-full object-cover"
								onerror={() => (posterFailed = true)}
							/>
						{:else}
							<div class="text-muted-foreground flex flex-col items-center gap-1.5 text-xs">
								<ImageIcon class="size-6" />
								暂无海报
							</div>
						{/if}
					</div>
					<div class="min-w-0 flex-1 space-y-2">
						<a
							href={preview.url}
							target="_blank"
							rel="noopener noreferrer"
							class="text-sm leading-snug font-semibold break-words hover:underline sm:text-base"
							>{preview.title}</a
						>
						<div class="flex flex-wrap gap-1.5">
							{#if preview.status}<Badge variant="secondary" class="max-w-full whitespace-normal"
									>{preview.status}</Badge
								>{/if}
							{#if preview.rating !== null}<Badge variant="secondary"
									>评分 {preview.rating.toLocaleString('zh-CN', {
										maximumFractionDigits: 1
									})}</Badge
								>{/if}
						</div>
						<p class="text-muted-foreground text-xs leading-relaxed">
							{#if preview.totalEpisodes !== null}共 {preview.totalEpisodes} 集 ·
							{:else}总集数待定 ·
							{/if}已上架 {preview.publishedEpisodes} 集
						</p>
						{#if preview.episode}
							<p class="text-xs leading-relaxed break-words">
								<span class="text-muted-foreground"
									>{wholeSeason ? '链接对应：' : '下载单集：'}</span
								>{episodeTitle(preview.episode)}
							</p>
						{/if}
					</div>
				</div>
				<dl class="grid grid-cols-2 gap-3 border-t pt-3">
					<div>
						<dt class="text-muted-foreground text-xs">全季累计播放</dt>
						<dd class="mt-1 text-sm font-semibold tabular-nums">{formatCount(preview.views)}</dd>
					</div>
					<div>
						<dt class="text-muted-foreground text-xs">本季追番</dt>
						<dd class="mt-1 text-sm font-semibold tabular-nums">
							{formatCount(preview.followers)}
						</dd>
					</div>
				</dl>
				{#if preview.followText}<p class="text-muted-foreground text-xs">
						{preview.followText}
					</p>{/if}
				{#if preview.description}<p
						class="text-muted-foreground line-clamp-3 text-xs leading-relaxed break-words"
					>
						{preview.description}
					</p>{/if}
				<p class="text-muted-foreground text-[11px]">
					统计来自 B 站；播放量为全季累计，本季追番与系列追番分别统计。
				</p>
			</section>
		{/if}
	</div>
{/if}
