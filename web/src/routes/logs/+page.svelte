<script lang="ts">
	import api from '$lib/api';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { onMount } from 'svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { toast } from 'svelte-sonner';

	let logEventSource: EventSource | null = null;
	let logs: Array<{ timestamp: string; level: string; message: string }> = [];
	let shouldAutoScroll = true;

	function checkScrollPosition() {
		const { scrollTop, scrollHeight, clientHeight } = document.documentElement;
		shouldAutoScroll = scrollTop + clientHeight >= scrollHeight - 5;
	}

	function scrollToBottom() {
		if (shouldAutoScroll) {
			window.scrollTo({ top: document.documentElement.scrollHeight, behavior: 'smooth' });
		}
	}

	function startLogStream() {
		if (logEventSource) {
			logEventSource.close();
		}
		logEventSource = api.createLogStream(
			(data: string) => {
				logs = [...logs.slice(-200), JSON.parse(data)];
				setTimeout(scrollToBottom, 0);
			},
			(error: Event) => {
				console.error('日志流错误:', error);
				toast.error('日志流出现错误，请稍后重试');
			}
		);
	}

	function stopLogStream() {
		if (logEventSource) {
			logEventSource.close();
			logEventSource = null;
		}
	}

	onMount(() => {
		setBreadcrumb([{ label: '日志' }]);
		window.addEventListener('scroll', checkScrollPosition);
		startLogStream();
		return () => {
			stopLogStream();
			window.removeEventListener('scroll', checkScrollPosition);
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
