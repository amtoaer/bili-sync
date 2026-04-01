<script lang="ts">
	import { onMount } from 'svelte';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { FolderIcon, SquarePenIcon, Trash2Icon } from '@lucide/svelte/icons';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import type { ApiError, YoutubeSource } from '$lib/types';

	let sources: YoutubeSource[] = [];
	let loading = false;
	let showEditDialog = false;
	let editingSource: YoutubeSource | null = null;
	let editForm = {
		path: '',
		enabled: false
	};
	let saving = false;

	async function loadSources() {
		loading = true;
		try {
			const response = await api.getYoutubeSources();
			sources = response.data.sources;
		} catch (error) {
			console.error('加载 YouTube 视频源失败：', error);
			toast.error('加载 YouTube 视频源失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function openEditDialog(source: YoutubeSource) {
		editingSource = source;
		editForm = {
			path: source.path,
			enabled: source.enabled
		};
		showEditDialog = true;
	}

	async function saveEdit() {
		if (!editingSource) return;
		saving = true;
		try {
			await api.updateYoutubeChannel(editingSource.id, editForm);
			toast.success('保存成功');
			showEditDialog = false;
			await loadSources();
		} catch (error) {
			console.error('保存 YouTube 视频源失败：', error);
			toast.error('保存失败', {
				description: (error as ApiError).message
			});
		} finally {
			saving = false;
		}
	}

	async function removeSource(source: YoutubeSource) {
		const sourceLabel = source.sourceType === 'playlist' ? '播放列表' : '频道';
		if (!confirm(`确定要删除${sourceLabel}「${source.name}」吗？`)) return;
		try {
			await api.removeYoutubeChannel(source.id);
			toast.success('删除成功');
			await loadSources();
		} catch (error) {
			console.error('删除 YouTube 视频源失败：', error);
			toast.error('删除失败', {
				description: (error as ApiError).message
			});
		}
	}

	function getSourceTypeLabel(sourceType: YoutubeSource['sourceType']) {
		return sourceType === 'playlist' ? '播放列表' : '频道';
	}

	onMount(() => {
		setBreadcrumb([{ label: 'YouTube 视频源' }]);
		loadSources();
	});
</script>

<svelte:head>
	<title>YouTube 视频源 - Bili Sync</title>
</svelte:head>

<div class="space-y-6">
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if sources.length > 0}
		<div class="overflow-x-auto">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head class="w-[12%]">类型</Table.Head>
						<Table.Head class="w-[24%]">名称</Table.Head>
						<Table.Head class="w-[31%]">下载路径</Table.Head>
						<Table.Head class="w-[15%]">最近同步</Table.Head>
						<Table.Head class="w-[8%]">状态</Table.Head>
						<Table.Head class="w-[10%] text-right">操作</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each sources as source (source.id)}
						<Table.Row>
							<Table.Cell>
								<Badge variant="secondary">{getSourceTypeLabel(source.sourceType)}</Badge>
							</Table.Cell>
							<Table.Cell>
								<div class="space-y-1">
									<div class="font-medium">{source.name}</div>
									<div class="text-muted-foreground text-xs break-all">{source.url}</div>
								</div>
							</Table.Cell>
							<Table.Cell>
								<div class="bg-secondary flex w-fit items-center gap-2 rounded-md px-2.5 py-1.5">
									<FolderIcon class="text-foreground/70 h-3.5 w-3.5 shrink-0" />
									<span class="text-foreground/70 font-mono text-xs font-medium select-text">
										{source.path}
									</span>
								</div>
							</Table.Cell>
							<Table.Cell>{source.latestPublishedAt || '-'}</Table.Cell>
							<Table.Cell>{source.enabled ? '已启用' : '已禁用'}</Table.Cell>
							<Table.Cell class="space-x-2 text-right">
								<Button size="sm" variant="outline" onclick={() => openEditDialog(source)}>
									<SquarePenIcon class="h-3 w-3" />
								</Button>
								<Button size="sm" variant="outline" onclick={() => removeSource(source)}>
									<Trash2Icon class="h-3 w-3" />
								</Button>
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{:else}
		<div class="flex items-center justify-center py-12">
			<div class="space-y-2 text-center">
				<p class="text-muted-foreground">暂无 YouTube 视频源</p>
				<p class="text-muted-foreground text-sm">
					到“订阅频道”“我的播放列表”或“手动提交链接”页面添加后，会显示在这里。
				</p>
			</div>
		</div>
	{/if}

	<Dialog.Root bind:open={showEditDialog}>
		<Dialog.Content class="max-w-lg">
			<Dialog.Title>编辑 YouTube 视频源</Dialog.Title>
			<div class="mt-6 space-y-6">
				<div>
					<Label for="youtube-edit-path" class="text-sm font-medium">下载路径</Label>
					<Input
						id="youtube-edit-path"
						type="text"
						bind:value={editForm.path}
						placeholder="请输入下载路径"
						class="mt-2"
					/>
				</div>

				<div class="flex items-center space-x-2">
					<Switch bind:checked={editForm.enabled} />
					<Label class="text-sm font-medium">启用此视频源</Label>
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
</div>
