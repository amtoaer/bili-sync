<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import {
		SquarePenIcon,
		FolderIcon,
		HeartIcon,
		UserIcon,
		ClockIcon,
		PlusIcon,
		InfoIcon,
		Trash2Icon,
		CircleCheckBigIcon,
		CircleXIcon
	} from '@lucide/svelte/icons';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import type { ApiError, VideoSourceDetail, VideoSourcesDetailsResponse, Rule } from '$lib/types';
	import api from '$lib/api';
	import RuleEditor from '$lib/components/rule-editor.svelte';
	import ListRestartIcon from '@lucide/svelte/icons/list-restart';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';

	let videoSourcesData: VideoSourcesDetailsResponse | null = null;
	let loading = false;
	let activeTab = 'favorites';

	// 添加对话框状态
	let showAddDialog = false;
	let addDialogType: 'favorites' | 'collections' | 'submissions' = 'favorites';
	let adding = false;

	// 编辑对话框状态
	let showEditDialog = false;
	let editingSource: VideoSourceDetail | null = null;
	let editingType = '';
	let editingIdx: number = 0;
	let saving = false;

	// 规则评估对话框状态
	let showEvaluateDialog = false;
	let evaluateSource: VideoSourceDetail | null = null;
	let evaluateType = '';
	let evaluating = false;

	// 删除对话框状态
	let showRemoveDialog = false;
	let removeSource: VideoSourceDetail | null = null;
	let removeType = '';
	let removeIdx: number = 0;
	let removing = false;

	// 编辑表单数据
	let editForm = {
		path: '',
		enabled: false,
		rule: null as Rule | null,
		useDynamicApi: null as boolean | null
	};

	// 表单数据
	let favoriteForm = { fid: '', path: '' };
	let collectionForm = { sid: '', mid: '', collection_type: '2', path: '' }; // 默认为合集
	let submissionForm = { upper_id: '', path: '' };

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

	// 打开编辑对话框
	function openEditDialog(type: string, source: VideoSourceDetail, idx: number) {
		editingSource = source;
		editingType = type;
		editingIdx = idx;
		editForm = {
			path: source.path,
			enabled: source.enabled,
			useDynamicApi: source.useDynamicApi,
			rule: source.rule
		};
		showEditDialog = true;
	}

	function openEvaluateRules(type: string, source: VideoSourceDetail) {
		evaluateSource = source;
		evaluateType = type;
		showEvaluateDialog = true;
	}

	function openRemoveDialog(type: string, source: VideoSourceDetail, idx: number) {
		removeSource = source;
		removeType = type;
		removeIdx = idx;
		showRemoveDialog = true;
	}

	// 保存编辑
	async function saveEdit() {
		if (!editingSource) return;

		if (!editForm.path?.trim()) {
			toast.error('路径不能为空');
			return;
		}
		saving = true;
		try {
			let response = await api.updateVideoSource(editingType, editingSource.id, {
				path: editForm.path,
				enabled: editForm.enabled,
				rule: editForm.rule,
				useDynamicApi: editForm.useDynamicApi
			});
			// 更新本地数据
			if (videoSourcesData && editingSource) {
				const sources = videoSourcesData[
					editingType as keyof VideoSourcesDetailsResponse
				] as VideoSourceDetail[];
				sources[editingIdx] = {
					...sources[editingIdx],
					path: editForm.path,
					enabled: editForm.enabled,
					rule: editForm.rule,
					useDynamicApi: editForm.useDynamicApi,
					ruleDisplay: response.data.ruleDisplay
				};
				videoSourcesData = { ...videoSourcesData };
			}
			showEditDialog = false;
			toast.success('保存成功');
		} catch (error) {
			toast.error('保存失败', {
				description: (error as ApiError).message
			});
		} finally {
			saving = false;
		}
	}

	async function evaluateRules() {
		if (!evaluateSource) return;
		evaluating = true;
		try {
			let response = await api.evaluateVideoSourceRules(evaluateType, evaluateSource.id);
			if (response && response.data) {
				showEvaluateDialog = false;
				toast.success('重新评估规则成功');
			} else {
				toast.error('重新评估规则失败');
			}
		} catch (error) {
			toast.error('重新评估规则失败', {
				description: (error as ApiError).message
			});
		} finally {
			evaluating = false;
		}
	}

	async function removeVideoSource() {
		if (!removeSource) return;
		removing = true;
		try {
			let response = await api.removeVideoSource(removeType, removeSource.id);
			if (response && response.data) {
				if (videoSourcesData) {
					const sources = videoSourcesData[
						removeType as keyof VideoSourcesDetailsResponse
					] as VideoSourceDetail[];
					sources.splice(removeIdx, 1);
					videoSourcesData = { ...videoSourcesData };
				}
				showRemoveDialog = false;
				toast.success('删除视频源成功');
			} else {
				toast.error('删除视频源失败');
			}
		} catch (error) {
			toast.error('删除视频源失败', {
				description: (error as ApiError).message
			});
		} finally {
			removing = false;
		}
	}

	function getSourcesForTab(tabValue: string): VideoSourceDetail[] {
		if (!videoSourcesData) return [];
		return videoSourcesData[tabValue as keyof VideoSourcesDetailsResponse] as VideoSourceDetail[];
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
					<Tabs.Trigger value={key} class="relative">
						{config.label}
					</Tabs.Trigger>
				{/each}
			</Tabs.List>
			{#each Object.entries(TAB_CONFIG) as [key, config] (key)}
				{@const sources = getSourcesForTab(key)}
				<Tabs.Content value={key} class="mt-6">
					<div class="mb-4 flex items-center justify-between">
						<div></div>
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
										<Table.Head class="w-[20%]">名称</Table.Head>
										<Table.Head class="w-[30%]">下载路径</Table.Head>
										<Table.Head class="w-[15%]">过滤规则</Table.Head>
										<Table.Head class="w-[15%]">启用状态</Table.Head>
										<Table.Head class="w-[10%] text-right">操作</Table.Head>
									</Table.Row>
								</Table.Header>
								<Table.Body>
									{#each sources as source, index (index)}
										<Table.Row>
											<Table.Cell class="font-medium">{source.name}</Table.Cell>
											<Table.Cell>
												<div
													class="bg-secondary hover:bg-secondary/80 flex w-fit cursor-text items-center gap-2 rounded-md px-2.5 py-1.5 transition-colors"
												>
													<FolderIcon class="text-foreground/70 h-3.5 w-3.5 shrink-0" />
													<span
														class="text-foreground/70 font-mono text-xs font-medium select-text"
													>
														{source.path || '未设置'}
													</span>
												</div>
											</Table.Cell>
											<Table.Cell>
												{#if source.rule && source.rule.length > 0}
													<Tooltip.Root disableHoverableContent={true}>
														<Tooltip.Trigger>
															<Badge
																variant="secondary"
																class="flex w-fit cursor-help items-center gap-1.5"
															>
																{source.rule.length} 条规则
															</Badge>
														</Tooltip.Trigger>
														<Tooltip.Content>
															<p class="text-xs">{source.ruleDisplay}</p>
														</Tooltip.Content>
													</Tooltip.Root>
												{:else}
													<Badge variant="secondary" class="flex w-fit items-center gap-1.5">
														-
													</Badge>
												{/if}
											</Table.Cell>
											<Table.Cell>
												{#if source.enabled}
													<Badge
														class="flex w-fit items-center gap-1.5 bg-emerald-700 text-emerald-100"
													>
														<CircleCheckBigIcon class="h-3 w-3" />
														已启用{#if key === 'submissions' && source.useDynamicApi !== null}{source.useDynamicApi
																? '（动态 API）'
																: ''}{/if}
													</Badge>
												{:else}
													<Badge class="flex w-fit items-center gap-1.5 bg-rose-700 text-rose-100 ">
														<CircleXIcon class="h-3 w-3" />
														已禁用
													</Badge>
												{/if}
											</Table.Cell>

											<Table.Cell class="text-right">
												<Tooltip.Root disableHoverableContent={true}>
													<Tooltip.Trigger>
														<Button
															size="sm"
															variant="outline"
															onclick={() => openEditDialog(key, source, index)}
															class="h-8 w-8 p-0"
														>
															<SquarePenIcon class="h-3 w-3" />
														</Button>
													</Tooltip.Trigger>
													<Tooltip.Content>
														<p class="text-xs">编辑</p>
													</Tooltip.Content>
												</Tooltip.Root>
												<Tooltip.Root disableHoverableContent={true}>
													<Tooltip.Trigger>
														<Button
															size="sm"
															variant="outline"
															onclick={() => openEvaluateRules(key, source)}
															class="h-8 w-8 p-0"
														>
															<ListRestartIcon class="h-3 w-3" />
														</Button>
													</Tooltip.Trigger>
													<Tooltip.Content>
														<p class="text-xs">重新评估规则</p>
													</Tooltip.Content>
												</Tooltip.Root>
												{#if activeTab !== 'watch_later'}
													<Tooltip.Root disableHoverableContent={true}>
														<Tooltip.Trigger>
															<Button
																size="sm"
																variant="outline"
																onclick={() => openRemoveDialog(key, source, index)}
																class="h-8 w-8 p-0"
															>
																<Trash2Icon class="h-3 w-3" />
															</Button>
														</Tooltip.Trigger>
														<Tooltip.Content>
															<p class="text-xs">删除</p>
														</Tooltip.Content>
													</Tooltip.Root>
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

	<!-- 编辑对话框 -->
	<Dialog.Root bind:open={showEditDialog}>
		<Dialog.Content
			class="no-scrollbar max-h-[85vh] max-w-[90vw]! overflow-y-auto lg:max-w-[70vw]!"
		>
			<Dialog.Title class="text-lg font-semibold">
				编辑视频源: {editingSource?.name || ''}
			</Dialog.Title>
			<div class="mt-6 space-y-6">
				<!-- 下载路径 -->
				<div>
					<Label for="edit-path" class="text-sm font-medium">下载路径</Label>
					<Input
						id="edit-path"
						type="text"
						bind:value={editForm.path}
						placeholder="请输入下载路径，例如：/path/to/download"
						class="mt-2"
					/>
				</div>

				<!-- 启用状态 -->
				<div class="flex items-center space-x-2">
					<Switch bind:checked={editForm.enabled} />
					<Label class="text-sm font-medium">启用此视频源</Label>
				</div>

				{#if editingType === 'submissions' && editForm.useDynamicApi !== null}
					<div class="flex items-center space-x-2">
						<Switch bind:checked={editForm.useDynamicApi} />
						<div class="flex items-center gap-1">
							<Label class="text-sm font-medium">使用动态 API 获取视频</Label>
							<Tooltip.Root>
								<Tooltip.Trigger>
									<InfoIcon class="text-muted-foreground h-3.5 w-3.5" />
								</Tooltip.Trigger>
								<Tooltip.Content>
									<p class="text-xs">
										只有使用动态 API 才能拉取到动态视频，但该接口不提供分页参数，每次请求只能拉取 12
										条视频。<br />这会一定程度上增加请求次数，用户可根据实际情况酌情选择，推荐仅在
										UP 主有较多动态视频时开启。
									</p>
								</Tooltip.Content>
							</Tooltip.Root>
						</div>
					</div>
				{/if}

				<!-- 规则编辑器 -->
				<div>
					<RuleEditor rule={editForm.rule} onRuleChange={(rule) => (editForm.rule = rule)} />
				</div>
			</div>
			<div class="mt-8 flex justify-end gap-3">
				<Button variant="outline" onclick={() => (showEditDialog = false)} disabled={saving}>
					取消
				</Button>
				<Button onclick={saveEdit} disabled={saving}>
					{saving ? '保存中...' : '保存'}
				</Button>
			</div>
		</Dialog.Content>
	</Dialog.Root>

	<AlertDialog.Root bind:open={showEvaluateDialog}>
		<AlertDialog.Content>
			<AlertDialog.Header>
				<AlertDialog.Title>重新评估规则</AlertDialog.Title>
				<AlertDialog.Description>
					确定要重新评估视频源 <strong>"{evaluateSource?.name}"</strong> 的过滤规则吗？<br />
					规则修改后默认仅对新视频生效，该操作可使用当前规则对数据库中已存在的历史视频进行重新评估，<span
						class="text-destructive font-medium">无法撤销</span
					>。<br />
				</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				<AlertDialog.Cancel
					disabled={evaluating}
					onclick={() => {
						showEvaluateDialog = false;
					}}>取消</AlertDialog.Cancel
				>
				<AlertDialog.Action onclick={evaluateRules} disabled={evaluating}>
					{evaluating ? '重新评估中' : '确认重新评估'}
				</AlertDialog.Action>
			</AlertDialog.Footer>
		</AlertDialog.Content>
	</AlertDialog.Root>

	<AlertDialog.Root bind:open={showRemoveDialog}>
		<AlertDialog.Content>
			<AlertDialog.Header>
				<AlertDialog.Title>删除视频源</AlertDialog.Title>
				<AlertDialog.Description>
					确定要删除视频源 <strong>"{removeSource?.name}"</strong> 吗？<br />
					删除后该视频源相关的所有条目将从数据库中移除（不影响磁盘文件），该操作<span
						class="text-destructive font-medium">无法撤销</span
					>。<br />
				</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				<AlertDialog.Cancel
					disabled={removing}
					onclick={() => {
						showRemoveDialog = false;
					}}>取消</AlertDialog.Cancel
				>
				<AlertDialog.Action onclick={removeVideoSource} disabled={removing}>
					{removing ? '删除中' : '删除'}
				</AlertDialog.Action>
			</AlertDialog.Footer>
		</AlertDialog.Content>
	</AlertDialog.Root>

	<!-- 添加对话框 -->
	<Dialog.Root bind:open={showAddDialog}>
		<Dialog.Content>
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
								class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring mt-1 flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
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
