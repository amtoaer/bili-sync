<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import EditIcon from '@lucide/svelte/icons/edit';
	import SaveIcon from '@lucide/svelte/icons/save';
	import XIcon from '@lucide/svelte/icons/x';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import HeartIcon from '@lucide/svelte/icons/heart';
	import UserIcon from '@lucide/svelte/icons/user';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { goto } from '$app/navigation';
	import { appStateStore, ToQuery } from '$lib/stores/filter';
	import type { ApiError, VideoSourceDetail, VideoSourcesDetailsResponse } from '$lib/types';
	import api from '$lib/api';

	let videoSourcesData: VideoSourcesDetailsResponse | null = null;
	let loading = false;
	let activeTab = 'favorites';

	type ExtendedVideoSource = VideoSourceDetail & {
		type: string;
		originalIndex: number;
		editing?: boolean;
		editingPath?: string;
		editingEnabled?: boolean;
	};

	const TAB_CONFIG = {
		favorites: { label: '收藏夹', icon: HeartIcon, color: 'bg-red-500' },
		collections: { label: '合集 / 列表', icon: FolderIcon, color: 'bg-blue-500' },
		submissions: { label: '用户投稿', icon: UserIcon, color: 'bg-green-500' },
		watch_later: { label: '稍后再看', icon: ClockIcon, color: 'bg-yellow-500' }
	} as const;

	// 数据加载
	async function loadVideoSources() {
		loading = true;
		try {
			const response = await api.getVideoSourcesDetails();
			videoSourcesData = response.data;
		} catch (error) {
			toast.error('加载视频源失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function startEdit(type: string, index: number) {
		if (!videoSourcesData) return;
		const sources = videoSourcesData[type as keyof VideoSourcesDetailsResponse];
		if (!sources?.[index]) return;

		const source = sources[index] as any;
		source.editing = true;
		source.editingPath = source.path;
		source.editingEnabled = source.enabled;
		videoSourcesData = { ...videoSourcesData };
	}

	function cancelEdit(type: string, index: number) {
		if (!videoSourcesData) return;
		const sources = videoSourcesData[type as keyof VideoSourcesDetailsResponse];
		if (!sources?.[index]) return;

		const source = sources[index] as any;
		source.editing = false;
		source.editingPath = undefined;
		source.editingEnabled = undefined;
		videoSourcesData = { ...videoSourcesData };
	}

	async function saveEdit(type: string, index: number) {
		if (!videoSourcesData) return;
		const sources = videoSourcesData[type as keyof VideoSourcesDetailsResponse];
		if (!sources?.[index]) return;

		const source = sources[index] as any;
		if (!source.editingPath?.trim()) {
			toast.error('路径不能为空');
			return;
		}

		try {
			await api.updateVideoSource(type, source.id, {
				path: source.editingPath,
				enabled: source.editingEnabled ?? false
			});

			source.path = source.editingPath;
			source.enabled = source.editingEnabled ?? false;
			source.editing = false;
			source.editingPath = undefined;
			source.editingEnabled = undefined;
			videoSourcesData = { ...videoSourcesData };

			toast.success('保存成功');
		} catch (error) {
			toast.error('保存失败', {
				description: (error as ApiError).message
			});
		}
	}

	function getSourcesForTab(tabValue: string): ExtendedVideoSource[] {
		if (!videoSourcesData) return [];
		const sources = videoSourcesData[
			tabValue as keyof VideoSourcesDetailsResponse
		] as VideoSourceDetail[];
		// 直接返回原始数据的引用，只添加必要的属性
		return sources.map((source, originalIndex) => {
			(source as any).type = tabValue;
			(source as any).originalIndex = originalIndex;
			return source as ExtendedVideoSource;
		});
	}

	// 初始化
	onMount(() => {
		setBreadcrumb([
			{
				label: '主页',
				onClick: () => {
					goto(`/${ToQuery($appStateStore)}`);
				}
			},
			{ label: '视频源管理', isActive: true }
		]);
		loadVideoSources();
	});
</script>

<svelte:head>
	<title>视频源管理 - Bili Sync</title>
</svelte:head>

<div class="max-w-6xl">
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if videoSourcesData}
		<Tabs.Root bind:value={activeTab} class="w-full">
			<Tabs.List class="grid h-12 w-full grid-cols-4 bg-transparent p-0">
				{#each Object.entries(TAB_CONFIG) as [key, config]}
					{@const sources = getSourcesForTab(key)}
					<Tabs.Trigger
						value={key}
						class="data-[state=active]:bg-muted/50 data-[state=active]:text-foreground text-muted-foreground hover:bg-muted/30 hover:text-foreground mx-1 flex min-w-0 items-center justify-center gap-2 rounded-lg bg-transparent px-2 py-3 text-sm font-medium transition-all sm:px-4"
					>
						<div
							class="flex h-5 w-5 items-center justify-center rounded-full {config.color} flex-shrink-0"
						>
							<svelte:component this={config.icon} class="h-3 w-3 text-white" />
						</div>
						<span class="hidden truncate sm:inline">{config.label}</span>
						<span
							class="bg-background/50 flex-shrink-0 rounded-full px-2 py-0.5 text-xs font-medium"
							>{sources.length}</span
						>
					</Tabs.Trigger>
				{/each}
			</Tabs.List>
			{#each Object.entries(TAB_CONFIG) as [key, config]}
				{@const sources = getSourcesForTab(key)}
				<Tabs.Content value={key} class="mt-6">
					{#if sources.length > 0}
						<div class="overflow-x-auto">
							<Table.Root>
								<Table.Header>
									<Table.Row>
										<Table.Head class="w-[30%] md:w-[25%]">名称</Table.Head>
										<Table.Head class="w-[30%] md:w-[40%]">下载路径</Table.Head>
										<Table.Head class="w-[25%] md:w-[20%]">状态</Table.Head>
										<Table.Head class="w-[15%] text-right sm:w-[12%]">操作</Table.Head>
									</Table.Row>
								</Table.Header>
								<Table.Body>
									{#each sources as source, index}
										<Table.Row>
											<Table.Cell class="w-[30%] font-medium md:w-[25%]">{source.name}</Table.Cell>
											<Table.Cell class="w-[30%] md:w-[40%]">
												{#if source.editing}
													<input
														bind:value={source.editingPath}
														class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-8 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
														placeholder="输入下载路径"
													/>
												{:else}
													<code
														class="bg-muted text-muted-foreground inline-flex h-8 items-center rounded px-3 py-1 text-sm"
													>
														{source.path || '未设置'}
													</code>
												{/if}
											</Table.Cell>
											<Table.Cell class="w-[25%] md:w-[20%]">
												{#if source.editing}
													<div class="flex h-8 items-center">
														<Switch bind:checked={source.editingEnabled} />
													</div>
												{:else}
													<div class="flex h-8 items-center gap-2">
														<Switch checked={source.enabled} disabled />
														<span class="text-muted-foreground whitespace-nowrap text-sm">
															{source.enabled ? '已启用' : '已禁用'}
														</span>
													</div>
												{/if}
											</Table.Cell>
											<Table.Cell class="w-[15%] text-right sm:w-[12%]">
												{#if source.editing}
													<div
														class="flex flex-col items-end justify-end gap-1 sm:flex-row sm:items-center"
													>
														<Button
															size="sm"
															variant="outline"
															onclick={() => saveEdit(key, source.originalIndex)}
															class="h-7 w-7 p-0 sm:h-8 sm:w-8"
															title="保存"
														>
															<SaveIcon class="h-3 w-3" />
														</Button>
														<Button
															size="sm"
															variant="outline"
															onclick={() => cancelEdit(key, source.originalIndex)}
															class="h-7 w-7 p-0 sm:h-8 sm:w-8"
															title="取消"
														>
															<XIcon class="h-3 w-3" />
														</Button>
													</div>
												{:else}
													<Button
														size="sm"
														variant="outline"
														onclick={() => startEdit(key, source.originalIndex)}
														class="h-7 w-7 p-0 sm:h-8 sm:w-8"
														title="编辑"
													>
														<EditIcon class="h-3 w-3" />
													</Button>
												{/if}
											</Table.Cell>
										</Table.Row>
									{/each}
								</Table.Body>
							</Table.Root>
						</div>
					{:else}
						<div class="flex flex-col items-center justify-center py-12">
							<div
								class="flex h-12 w-12 items-center justify-center rounded-full {config.color} mb-4"
							>
								<svelte:component this={config.icon} class="h-6 w-6 text-white" />
							</div>
							<div class="text-muted-foreground mb-2">暂无{config.label}</div>
							<p class="text-muted-foreground text-sm">
								请先添加{config.label}订阅
							</p>
						</div>
					{/if}
				</Tabs.Content>
			{/each}
		</Tabs.Root>
	{:else}
		<div class="flex flex-col items-center justify-center py-12">
			<div class="text-muted-foreground mb-2">加载失败</div>
			<p class="text-muted-foreground text-sm">请刷新页面重试</p>
			<Button class="mt-4" onclick={loadVideoSources}>重新加载</Button>
		</div>
	{/if}
</div>
