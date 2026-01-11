<script lang="ts">
	import api from '$lib/api';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { onMount } from 'svelte';
	import { Badge } from '$lib/components/ui/badge';

	let unsubscribeLog: (() => void) | null = null;
	let logs: Array<{ timestamp: string; level: string; message: string }> = [];
	let shouldAutoScroll = true;
	let main: HTMLElement | null = null;

	function checkScrollPosition() {
		if (main) {
			const { scrollTop, scrollHeight, clientHeight } = main;
			shouldAutoScroll = scrollTop + clientHeight >= scrollHeight - 5;
		}
	}

	function scrollToBottom() {
		if (shouldAutoScroll && main) {
			main.scrollTop = main.scrollHeight;
		}
	}

	onMount(() => {
		setBreadcrumb([{ label: '日志' }]);
		main = document.getElementById('main');
		main?.addEventListener('scroll', checkScrollPosition);
		unsubscribeLog = api.subscribeToLogs((data: string) => {
			logs = [...logs.slice(-499), JSON.parse(data)];
			setTimeout(scrollToBottom, 0);
		});
		return () => {
			main?.removeEventListener('scroll', checkScrollPosition);
			if (unsubscribeLog) {
				unsubscribeLog();
				unsubscribeLog = null;
			}
		};
	});

	function getLevelColor(level: string) {
		switch (level) {
			case 'ERROR':
				return 'text-rose-600';
			case 'WARN':
				return 'text-yellow-600';
			case 'INFO':
			default:
				return 'text-emerald-600';
		}
	}
</script>

<svelte:head>
	<title>日志 - Bili Sync</title>
</svelte:head>

<div class="space-y-1">
	{#each logs as log, index (index)}
		<div
			class="flex items-center gap-3 rounded-md p-1 font-mono text-xs {index % 2 === 0
				? 'bg-muted/50'
				: 'bg-background'}"
		>
			<span class="text-muted-foreground w-32 shrink-0">
				{log.timestamp}
			</span>
			<Badge
				class="w-16 shrink-0 justify-center {getLevelColor(log.level)} bg-primary/90 font-semibold"
			>
				{log.level}
			</Badge>
			<span class="flex-1 break-all">
				{log.message}
			</span>
		</div>
	{/each}
	{#if logs.length === 0}
		<div class="text-muted-foreground py-8 text-center">暂无日志记录</div>
	{/if}
</div>
