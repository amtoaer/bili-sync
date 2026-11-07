<script lang="ts">
	import { onMount } from 'svelte';
	import SubscriptionCard from '$lib/components/subscription-card.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import type { ApiError, Followed } from '$lib/types';

	let bangumi = $state<Extract<Followed, { type: 'bangumi' }>[]>([]);
	let totalCount = $state(0);
	let currentPage = $state(0);
	let loading = $state(true);
	let error = $state('');
	let requestVersion = 0;
	const pageSize = 20;
	const totalPages = $derived(Math.max(1, Math.ceil(totalCount / pageSize)));

	async function loadBangumi(page = currentPage) {
		const version = ++requestVersion;
		currentPage = page;
		loading = true;
		error = '';
		try {
			const response = await api.getFollowedBangumi(page + 1, pageSize);
			if (version !== requestVersion) return;
			const lastPage = Math.max(0, Math.ceil(response.data.total / pageSize) - 1);
			if (page > lastPage) {
				await loadBangumi(lastPage);
				return;
			}
			bangumi = response.data.bangumi;
			totalCount = response.data.total;
		} catch (cause) {
			if (version !== requestVersion) return;
			error = (cause as ApiError).message || '暂时无法获取追番列表，请稍后重试。';
		} finally {
			if (version === requestVersion) loading = false;
		}
	}

	onMount(() => {
		setBreadcrumb([{ label: '我订阅的番剧' }]);
		void loadBangumi(0);
		return () => {
			requestVersion++;
		};
	});
</script>

<svelte:head>
	<title>我订阅的番剧 - Bili Sync</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex items-start justify-between gap-3">
		<div class="space-y-2">
			<h1 class="text-lg font-semibold">我订阅的番剧</h1>
			<p class="text-muted-foreground text-sm leading-relaxed">
				来自当前 B 站账号的追番列表。点击「订阅」添加整季下载并持续同步新集。
			</p>
			<p class="text-muted-foreground text-xs">添加后，请前往「视频源 → 番剧」启用订阅。</p>
		</div>
		<Button variant="outline" size="sm" disabled={loading} onclick={() => loadBangumi()}>
			<RefreshCwIcon class="size-4" />刷新
		</Button>
	</div>

	{#if loading}
		<div class="text-muted-foreground py-12 text-center" role="status">正在加载追番列表…</div>
	{:else if error}
		<div class="bg-muted/30 space-y-3 rounded-lg border p-6 text-center" role="alert">
			<p class="font-medium">加载番剧列表失败</p>
			<p class="text-muted-foreground text-sm break-words">{error}</p>
			<p class="text-muted-foreground text-sm">请确认已在设置中登录 B 站账号。</p>
			<Button variant="outline" onclick={() => loadBangumi()}>重试</Button>
		</div>
	{:else if bangumi.length > 0}
		<p class="text-muted-foreground text-sm">
			共 {totalCount} 部番剧 · 第 {currentPage + 1} / {totalPages} 页
		</p>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
			{#each bangumi as item (item.season_id)}
				<SubscriptionCard {item} />
			{/each}
		</div>
		{#if totalPages > 1}
			<nav class="flex flex-wrap items-center justify-center gap-3" aria-label="番剧分页">
				<Button
					variant="outline"
					disabled={currentPage === 0}
					onclick={() => loadBangumi(currentPage - 1)}>上一页</Button
				>
				<span class="text-muted-foreground text-sm">{currentPage + 1} / {totalPages}</span>
				<Button
					variant="outline"
					disabled={currentPage >= totalPages - 1}
					onclick={() => loadBangumi(currentPage + 1)}>下一页</Button
				>
			</nav>
		{/if}
	{:else}
		<div class="space-y-2 py-12 text-center">
			<p class="text-muted-foreground">暂无订阅的番剧</p>
			<p class="text-muted-foreground text-sm">先在 B 站番剧页面点击「追番」，再回来刷新列表。</p>
		</div>
	{/if}
</div>
