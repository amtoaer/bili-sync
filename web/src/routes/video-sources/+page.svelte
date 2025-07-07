<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import EditIcon from '@lucide/svelte/icons/edit';
	import SaveIcon from '@lucide/svelte/icons/save';
	import XIcon from '@lucide/svelte/icons/x';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import HeartIcon from '@lucide/svelte/icons/heart';
	import UserIcon from '@lucide/svelte/icons/user';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { goto } from '$app/navigation';
	import { appStateStore, ToQuery } from '$lib/stores/filter';
	import type { ApiError, VideoSourceDetail, VideoSourcesDetailsResponse } from '$lib/types';
	import api from '$lib/api';

	let videoSourcesData: VideoSourcesDetailsResponse | null = null;
	let loading = false;
	let activeTab = 'favorites';

	// 添加对话框状态
	let showAddDialog = false;
	let addDialogType: 'favorites' | 'collections' | 'submissions' = 'favorites';
	let adding = false;

	// 表单数据
	let favoriteForm = { fid: '', path: '' };
	let collectionForm = { sid: '', mid: '', collection_type: '2', path: '' }; // 默认为合集
	let submissionForm = { upper_id: '', path: '' };

	type ExtendedVideoSource = VideoSourceDetail & {
		type: string;
		originalIndex: number;
		editing?: boolean;
		editingPath?: string;
		editingEnabled?: boolean;
	};

	const TAB_CONFIG = {
		favorites: { label: '收藏夹', icon: HeartIcon },
		collections: { label: '合集 / 列表', icon: FolderIcon },
		submissions: { label: '用户投稿', icon: UserIcon },
		watch_later: { label: '稍后再看', icon: ClockIcon }
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

		const source = sources[index] as ExtendedVideoSource;
		source.editing = true;
		source.editingPath = source.path;
		source.editingEnabled = source.enabled;
		videoSourcesData = { ...videoSourcesData };
	}

	function cancelEdit(type: string, index: number) {
		if (!videoSourcesData) return;
		const sources = videoSourcesData[type as keyof VideoSourcesDetailsResponse];
		if (!sources?.[index]) return;

		const source = sources[index] as ExtendedVideoSource;
		source.editing = false;
		source.editingPath = undefined;
		source.editingEnabled = undefined;
		videoSourcesData = { ...videoSourcesData };
	}

	async function saveEdit(type: string, index: number) {
		if (!videoSourcesData) return;
		const sources = videoSourcesData[type as keyof VideoSourcesDetailsResponse];
		if (!sources?.[index]) return;

		const source = sources[index] as ExtendedVideoSource;
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
			// 使用类型断言来扩展 VideoSourceDetail
			const extendedSource = source as ExtendedVideoSource;
			extendedSource.type = tabValue;
			extendedSource.originalIndex = originalIndex;
			return extendedSource;
		});
	}

	// 打开添加对话框
	function openAddDialog(type: 'favorites' | 'collections' | 'submissions') {
		addDialogType = type;
		// 重置表单
		favoriteForm = { fid: '', path: '' };
		collectionForm = { sid: '', mid: '', collection_type: '2', path: '' };
		submissionForm = { upper_id: '', path: '' };
		showAddDialog = true;
	}

	// 处理添加
	async function handleAdd() {
		adding = true;
		try {
			switch (addDialogType) {
				case 'favorites':
					if (!favoriteForm.fid || !favoriteForm.path.trim()) {
						toast.error('请填写完整的收藏夹信息');
						return;
					}
					await api.insertFavorite({
						fid: parseInt(favoriteForm.fid),
						path: favoriteForm.path
					});
					break;
				case 'collections':
					if (!collectionForm.sid || !collectionForm.mid || !collectionForm.path.trim()) {
						toast.error('请填写完整的合集信息');
						return;
					}
					await api.insertCollection({
						sid: parseInt(collectionForm.sid),
						mid: parseInt(collectionForm.mid),
						collection_type: parseInt(collectionForm.collection_type),
						path: collectionForm.path
					});
					break;
				case 'submissions':
					if (!submissionForm.upper_id || !submissionForm.path.trim()) {
						toast.error('请填写完整的用户投稿信息');
						return;
					}
					await api.insertSubmission({
						upper_id: parseInt(submissionForm.upper_id),
						path: submissionForm.path
					});
					break;
			}

			toast.success('添加成功');
			showAddDialog = false;
			loadVideoSources(); // 重新加载数据
		} catch (error) {
			toast.error('添加失败', {
				description: (error as ApiError).message
			});
		} finally {
			adding = false;
		}
	}

	// 初始化
	onMount(() => {
		setBreadcrumb([{ label: '视频源' }]);
		loadVideoSources();
	});
</script>

<svelte:head>
	<title>视频源管理 - Bili Sync</title>
</svelte:head>

<div class="space-y-6">
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if videoSourcesData}
		<Tabs.Root bind:value={activeTab} class="w-full">
			<Tabs.List class="grid w-full grid-cols-4">
				{#each Object.entries(TAB_CONFIG) as [key, config] (key)}
					{@const sources = getSourcesForTab(key)}
					<Tabs.Trigger value={key} class="relative">
						{config.label}（{sources.length}）
					</Tabs.Trigger>
				{/each}
			</Tabs.List>
			{#each Object.entries(TAB_CONFIG) as [key, config] (key)}
				{@const sources = getSourcesForTab(key)}
				<Tabs.Content value={key} class="mt-6">
					<div class="mb-4 flex items-center justify-between">
						<h3 class="text-lg font-medium">{config.label}管理</h3>
						{#if key === 'favorites' || key === 'collections' || key === 'submissions'}
							<Button size="sm" onclick={() => openAddDialog(key)} class="flex items-center gap-2">
								<PlusIcon class="h-4 w-4" />
								手动添加
							</Button>
						{/if}
					</div>
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
									{#each sources as source, index (index)}
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
							<svelte:component this={config.icon} class="text-muted-foreground mb-4 h-12 w-12" />
							<div class="text-muted-foreground mb-2 text-lg font-medium">暂无{config.label}</div>
							<p class="text-muted-foreground mb-4 text-center text-sm">
								{#if key === 'favorites'}
									还没有添加任何收藏夹订阅
								{:else if key === 'collections'}
									还没有添加任何合集或列表订阅
								{:else if key === 'submissions'}
									还没有添加任何用户投稿订阅
								{:else}
									还没有添加稍后再看订阅
								{/if}
							</p>
							{#if key === 'favorites' || key === 'collections' || key === 'submissions'}
								<Button onclick={() => openAddDialog(key)} class="flex items-center gap-2">
									<PlusIcon class="h-4 w-4" />
									手动添加
								</Button>
							{/if}
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

	<Dialog.Root bind:open={showAddDialog}>
		<Dialog.Overlay class="data-[state=open]:animate-overlay-show fixed inset-0 bg-black/30" />
		<Dialog.Content
			class="data-[state=open]:animate-content-show bg-background fixed left-1/2 top-1/2 z-50 max-h-[85vh] w-full max-w-3xl -translate-x-1/2 -translate-y-1/2 rounded-lg border p-6 shadow-md outline-none"
		>
			<Dialog.Title class="text-lg font-semibold">
				{#if addDialogType === 'favorites'}
					添加收藏夹
				{:else if addDialogType === 'collections'}
					添加合集
				{:else}
					添加用户投稿
				{/if}
			</Dialog.Title>
			<div class="mt-4">
				{#if addDialogType === 'favorites'}
					<div class="space-y-4">
						<div>
							<Label for="fid" class="text-sm font-medium">收藏夹ID (fid)</Label>
							<Input
								id="fid"
								type="number"
								bind:value={favoriteForm.fid}
								placeholder="请输入收藏夹ID"
								class="mt-1"
							/>
						</div>
					</div>
				{:else if addDialogType === 'collections'}
					<div class="space-y-4">
						<div>
							<Label for="collection-type" class="text-sm font-medium">合集类型</Label>
							<select
								id="collection-type"
								bind:value={collectionForm.collection_type}
								class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring mt-1 flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
							>
								<option value="1">列表 (Series)</option>
								<option value="2">合集 (Season)</option>
							</select>
						</div>
						<div class="grid grid-cols-2 gap-4">
							<div>
								<Label for="sid" class="text-sm font-medium">
									{collectionForm.collection_type === '1'
										? '列表ID (series_id)'
										: '合集ID (season_id)'}
								</Label>
								<Input
									id="sid"
									type="number"
									bind:value={collectionForm.sid}
									placeholder={collectionForm.collection_type === '1'
										? '请输入列表ID'
										: '请输入合集ID'}
									class="mt-1"
								/>
							</div>
							<div>
								<Label for="mid" class="text-sm font-medium">用户ID (mid)</Label>
								<Input
									id="mid"
									type="number"
									bind:value={collectionForm.mid}
									placeholder="请输入用户ID"
									class="mt-1"
								/>
							</div>
						</div>
						<p class="text-muted-foreground text-xs">可从合集/列表页面URL中获取相应ID</p>
					</div>
				{:else}
					<div class="space-y-4">
						<div>
							<Label for="upper_id" class="text-sm font-medium">UP主ID (mid)</Label>
							<Input
								id="upper_id"
								type="number"
								bind:value={submissionForm.upper_id}
								placeholder="请输入UP主ID"
								class="mt-1"
							/>
						</div>
					</div>
				{/if}
				<div class="mt-4">
					<Label for="path" class="text-sm font-medium">下载路径</Label>
					{#if addDialogType === 'favorites'}
						<Input
							id="path"
							type="text"
							bind:value={favoriteForm.path}
							placeholder="请输入下载路径，例如：/path/to/download"
							class="mt-1"
						/>
					{:else if addDialogType === 'collections'}
						<Input
							id="path"
							type="text"
							bind:value={collectionForm.path}
							placeholder="请输入下载路径，例如：/path/to/download"
							class="mt-1"
						/>
					{:else}
						<Input
							id="path"
							type="text"
							bind:value={submissionForm.path}
							placeholder="请输入下载路径，例如：/path/to/download"
							class="mt-1"
						/>
					{/if}
				</div>
			</div>
			<div class="mt-6 flex justify-end gap-2">
				<Button
					variant="outline"
					onclick={() => (showAddDialog = false)}
					disabled={adding}
					class="px-4"
				>
					取消
				</Button>
				<Button onclick={handleAdd} disabled={adding} class="px-4">
					{adding ? '添加中...' : '添加'}
				</Button>
			</div>
		</Dialog.Content>
	</Dialog.Root>
</div>
