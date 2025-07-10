<script lang="ts">
	import { onMount } from 'svelte';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Progress } from '$lib/components/ui/progress/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Chart from '$lib/components/ui/chart/index.js';
	import MyChartTooltip from '$lib/components/custom/my-chart-tooltip.svelte';
	import { curveNatural } from 'd3-shape';
	import { BarChart, AreaChart } from 'layerchart';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { toast } from 'svelte-sonner';
	import CloudDownloadIcon from '@lucide/svelte/icons/cloud-download';
	import api from '$lib/api';
	import type { DashBoardResponse, SysInfoResponse, ApiError } from '$lib/types';
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import HeartIcon from '@lucide/svelte/icons/heart';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import UserIcon from '@lucide/svelte/icons/user';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import VideoIcon from '@lucide/svelte/icons/video';
	import HardDriveIcon from '@lucide/svelte/icons/hard-drive';
	import CpuIcon from '@lucide/svelte/icons/cpu';
	import MemoryStickIcon from '@lucide/svelte/icons/memory-stick';
	import PlayIcon from '@lucide/svelte/icons/play';
	import CheckCircleIcon from '@lucide/svelte/icons/check-circle';
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import { taskStatusStore } from '$lib/stores/tasks';

	let dashboardData: DashBoardResponse | null = null;
	let sysInfo: SysInfoResponse | null = null;
	let loading = false;
	let sysInfoEventSource: EventSource | null = null;

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}

	function formatCpu(cpu: number): string {
		return `${cpu.toFixed(1)}%`;
	}

	async function loadDashboard() {
		loading = true;
		try {
			const response = await api.getDashboard();
			dashboardData = response.data;
		} catch (error) {
			console.error('加载仪表盘数据失败:', error);
			toast.error('加载仪表盘数据失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	// 启动系统信息流
	function startSysInfoStream() {
		sysInfoEventSource = api.createSysInfoStream(
			(data) => {
				sysInfo = data;
			},
			(error) => {
				console.error('系统信息流错误:', error);
				toast.error('系统信息流出现错误，请检查网络连接或稍后重试');
			}
		);
	}

	// 停止系统信息流
	function stopSysInfoStream() {
		if (sysInfoEventSource) {
			sysInfoEventSource.close();
			sysInfoEventSource = null;
		}
	}

	onMount(() => {
		setBreadcrumb([{ label: '仪表盘' }]);
		loadDashboard();
		startSysInfoStream();
		return () => {
			stopSysInfoStream();
		};
	});

	// 图表配置
	const videoChartConfig = {
		videos: {
			label: '视频数量',
			color: 'var(--color-slate-700)'
		}
	} satisfies Chart.ChartConfig;

	const memoryChartConfig = {
		used: {
			label: '整体占用',
			color: 'var(--color-slate-700)'
		},
		process: {
			label: '程序占用',
			color: 'var(--color-slate-950)'
		}
	} satisfies Chart.ChartConfig;

	const cpuChartConfig = {
		used: {
			label: '整体占用',
			color: 'var(--color-slate-700)'
		},
		process: {
			label: '程序占用',
			color: 'var(--color-slate-950)'
		}
	} satisfies Chart.ChartConfig;

	let memoryHistory: Array<{ time: Date; used: number; process: number }> = [];
	let cpuHistory: Array<{ time: Date; used: number; process: number }> = [];

	$: if (sysInfo) {
		memoryHistory = [
			...memoryHistory.slice(-19),
			{
				time: new Date(),
				used: sysInfo.used_memory,
				process: sysInfo.process_memory
			}
		];
		cpuHistory = [
			...cpuHistory.slice(-19),
			{
				time: new Date(),
				used: sysInfo.used_cpu,
				process: sysInfo.process_cpu
			}
		];
	}
	// 计算磁盘使用率
	$: diskUsagePercent = sysInfo
		? ((sysInfo.total_disk - sysInfo.available_disk) / sysInfo.total_disk) * 100
		: 0;
</script>

<svelte:head>
	<title>仪表盘 - Bili Sync</title>
</svelte:head>

<div class="space-y-6">
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else}
		<div class="grid gap-4 md:grid-cols-3">
			<Card class="md:col-span-1">
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">存储空间</CardTitle>
					<HardDriveIcon class="text-muted-foreground h-4 w-4" />
				</CardHeader>
				<CardContent>
					{#if sysInfo}
						<div class="space-y-2">
							<div class="flex items-center justify-between">
								<div class="text-2xl font-bold">{formatBytes(sysInfo.available_disk)} 可用</div>
								<div class="text-muted-foreground text-sm">
									共 {formatBytes(sysInfo.total_disk)}
								</div>
							</div>
							<Progress value={diskUsagePercent} class="h-2" />
							<div class="text-muted-foreground text-xs">
								已使用 {diskUsagePercent.toFixed(1)}% 的存储空间
							</div>
						</div>
					{:else}
						<div class="text-muted-foreground text-sm">加载中...</div>
					{/if}
				</CardContent>
			</Card>
			<Card class="md:col-span-2">
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">当前监听</CardTitle>
					<DatabaseIcon class="text-muted-foreground h-4 w-4" />
				</CardHeader>
				<CardContent>
					{#if dashboardData}
						<div class="grid grid-cols-2 gap-4">
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2">
									<HeartIcon class="text-muted-foreground h-4 w-4" />
									<span class="text-sm">收藏夹</span>
								</div>
								<Badge variant="outline">{dashboardData.enabled_favorites}</Badge>
							</div>
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2">
									<FolderIcon class="text-muted-foreground h-4 w-4" />
									<span class="text-sm">合集</span>
								</div>
								<Badge variant="outline">{dashboardData.enabled_collections}</Badge>
							</div>
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2">
									<UserIcon class="text-muted-foreground h-4 w-4" />
									<span class="text-sm">投稿</span>
								</div>
								<Badge variant="outline">{dashboardData.enabled_submissions}</Badge>
							</div>
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2">
									<ClockIcon class="text-muted-foreground h-4 w-4" />
									<span class="text-sm">稍后再看</span>
								</div>
								<Badge variant="outline">
									{dashboardData.enable_watch_later ? '启用' : '禁用'}
								</Badge>
							</div>
						</div>
					{:else}
						<div class="text-muted-foreground text-sm">加载中...</div>
					{/if}
				</CardContent>
			</Card>
		</div>

		<div class="grid gap-4 md:grid-cols-3">
			<Card class="max-w-full overflow-hidden md:col-span-2">
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">最近入库</CardTitle>
					<VideoIcon class="text-muted-foreground h-4 w-4" />
				</CardHeader>
				<CardContent>
					{#if dashboardData && dashboardData.videos_by_day.length > 0}
						<div class="mb-4 space-y-2">
							<div class="flex items-center justify-between text-sm">
								<span>近七日共新增视频</span>
								<span class="font-medium"
									>{dashboardData.videos_by_day.reduce((sum, v) => sum + v.cnt, 0)} 个</span
								>
							</div>
						</div>
						<Chart.Container config={videoChartConfig} class="h-[200px] w-full">
							<BarChart
								data={dashboardData.videos_by_day}
								x="day"
								axis="x"
								series={[
									{
										key: 'cnt',
										label: '新增视频',
										color: videoChartConfig.videos.color
									}
								]}
								props={{
									bars: {
										stroke: 'none',
										rounded: 'all',
										radius: 8,
										initialHeight: 0
									},
									highlight: { area: { fill: 'none' } },
									xAxis: { format: () => '' }
								}}
							>
								{#snippet tooltip()}
									<MyChartTooltip indicator="line" />
								{/snippet}
							</BarChart>
						</Chart.Container>
					{:else}
						<div class="text-muted-foreground flex h-[200px] items-center justify-center text-sm">
							暂无视频统计数据
						</div>
					{/if}</CardContent
				>
			</Card>
			<Card class="max-w-full md:col-span-1">
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">下载任务状态</CardTitle>
					<CloudDownloadIcon class="text-muted-foreground h-4 w-4" />
				</CardHeader>
				<CardContent>
					{#if $taskStatusStore}
						<div class="space-y-4">
							<div class="grid grid-cols-1 gap-6">
								<div class="mb-4 space-y-2">
									<div class="flex items-center justify-between text-sm">
										<span>当前任务状态</span>
										<Badge variant={$taskStatusStore.is_running ? 'default' : 'outline'}>
											{$taskStatusStore.is_running ? '运行中' : '未运行'}
										</Badge>
									</div>
								</div>
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-2">
										<PlayIcon class="text-muted-foreground h-4 w-4" />
										<span class="text-sm">开始运行</span>
									</div>
									<span class="text-muted-foreground text-sm">
										{$taskStatusStore.last_run
											? new Date($taskStatusStore.last_run).toLocaleString('en-US', {
													hour: '2-digit',
													minute: '2-digit',
													second: '2-digit',
													hour12: true
												})
											: '-'}
									</span>
								</div>
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-2">
										<CheckCircleIcon class="text-muted-foreground h-4 w-4" />
										<span class="text-sm">运行结束</span>
									</div>
									<span class="text-muted-foreground text-sm">
										{$taskStatusStore.last_finish
											? new Date($taskStatusStore.last_finish).toLocaleString('en-US', {
													hour: '2-digit',
													minute: '2-digit',
													second: '2-digit',
													hour12: true
												})
											: '-'}
									</span>
								</div>
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-2">
										<CalendarIcon class="text-muted-foreground h-4 w-4" />
										<span class="text-sm">下次运行</span>
									</div>
									<span class="text-muted-foreground text-sm">
										{$taskStatusStore.next_run
											? new Date($taskStatusStore.next_run).toLocaleString('en-US', {
													hour: '2-digit',
													minute: '2-digit',
													second: '2-digit',
													hour12: true
												})
											: '-'}
									</span>
								</div>
							</div>
						</div>
					{:else}
						<div class="text-muted-foreground text-sm">加载中...</div>
					{/if}
				</CardContent>
			</Card>
		</div>

		<!-- 第三行：系统监控 -->
		<div class="grid gap-4 md:grid-cols-2">
			<!-- 内存使用情况 -->
			<Card class="overflow-hidden">
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">内存使用情况</CardTitle>
					<MemoryStickIcon class="text-muted-foreground h-4 w-4" />
				</CardHeader>
				<CardContent>
					{#if sysInfo}
						<div class="mb-4 space-y-2">
							<div class="flex items-center justify-between text-sm">
								<span>当前内存使用</span>
								<span class="font-medium"
									>{formatBytes(sysInfo.used_memory)} / {formatBytes(sysInfo.total_memory)}</span
								>
							</div>
						</div>
					{/if}
					{#if memoryHistory.length > 0}
						<Chart.Container config={memoryChartConfig} class="h-[150px] w-full">
							<AreaChart
								data={memoryHistory}
								x="time"
								axis="x"
								series={[
									{
										key: 'used',
										label: memoryChartConfig.used.label,
										color: memoryChartConfig.used.color
									},
									{
										key: 'process',
										label: memoryChartConfig.process.label,
										color: memoryChartConfig.process.color
									}
								]}
								props={{
									area: {
										curve: curveNatural,
										line: { class: 'stroke-1' },
										'fill-opacity': 0.4
									},
									xAxis: {
										format: () => ''
									}
								}}
							>
								{#snippet tooltip()}
									<MyChartTooltip
										labelFormatter={(v: Date) => {
											return v.toLocaleString('en-US', {
												hour: '2-digit',
												minute: '2-digit',
												second: '2-digit',
												hour12: true
											});
										}}
										valueFormatter={(v: number) => formatBytes(v)}
										indicator="line"
									/>
								{/snippet}
							</AreaChart>
						</Chart.Container>
					{:else}
						<div class="text-muted-foreground flex h-[200px] items-center justify-center text-sm">
							等待数据...
						</div>
					{/if}
				</CardContent>
			</Card>

			<Card class="overflow-hidden">
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">CPU 使用情况</CardTitle>
					<CpuIcon class="text-muted-foreground h-4 w-4" />
				</CardHeader>
				<CardContent class="overflow-hidden">
					{#if sysInfo}
						<div class="mb-4 space-y-2">
							<div class="flex items-center justify-between text-sm">
								<span>当前 CPU 使用率</span>
								<span class="font-medium">{formatCpu(sysInfo.used_cpu)}</span>
							</div>
						</div>
					{/if}
					{#if cpuHistory.length > 0}
						<Chart.Container config={cpuChartConfig} class="h-[150px] w-full">
							<AreaChart
								data={cpuHistory}
								x="time"
								axis="x"
								series={[
									{
										key: 'used',
										label: cpuChartConfig.used.label,
										color: cpuChartConfig.used.color
									},
									{
										key: 'process',
										label: cpuChartConfig.process.label,
										color: cpuChartConfig.process.color
									}
								]}
								props={{
									area: {
										curve: curveNatural,
										line: { class: 'stroke-1' },
										'fill-opacity': 0.4
									},
									xAxis: {
										format: () => ''
									}
								}}
							>
								{#snippet tooltip()}
									<MyChartTooltip
										labelFormatter={(v: Date) => {
											return v.toLocaleString('en-US', {
												hour: '2-digit',
												minute: '2-digit',
												second: '2-digit',
												hour12: true
											});
										}}
										valueFormatter={(v: number) => formatCpu(v)}
										indicator="line"
									/>
								{/snippet}
							</AreaChart>
						</Chart.Container>
					{:else}
						<div class="text-muted-foreground flex h-[150px] items-center justify-center text-sm">
							等待数据...
						</div>
					{/if}
				</CardContent>
			</Card>
		</div>
	{/if}
</div>
