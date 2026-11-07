<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { toast } from 'svelte-sonner';
	import {
		Sheet,
		SheetContent,
		SheetDescription,
		SheetFooter,
		SheetHeader,
		SheetTitle
	} from '$lib/components/ui/sheet/index.js';
	import api from '$lib/api';
	import type {
		Followed,
		InsertFavoriteRequest,
		InsertCollectionRequest,
		InsertSubmissionRequest,
		ApiError
	} from '$lib/types';

	interface Props {
		open: boolean;
		item: Followed | null;
		onSuccess: (() => void) | null;
	}

	let { open = $bindable(false), item = null, onSuccess = null }: Props = $props();

	let customPath = $state('');
	let loading = $state(false);
	let pathEdited = $state(false);
	const pathType = $derived(
		item
			? {
					favorite: 'favorites',
					collection: 'collections',
					bangumi: 'bangumi',
					upper: 'submissions'
				}[item.type]
			: ''
	);

	function getTypeLabel(): string {
		if (!item) return '';

		switch (item.type) {
			case 'favorite':
				return '收藏夹';
			case 'collection':
				return '合集';
			case 'bangumi':
				return '番剧';
			case 'upper':
				return 'UP 主';
			default:
				return '';
		}
	}

	function getItemTitle(): string {
		if (!item) return '';

		switch (item.type) {
			case 'favorite':
			case 'collection':
			case 'bangumi':
				return item.title;
			case 'upper':
				return item.uname;
			default:
				return '';
		}
	}

	async function handleSubscribe() {
		if (loading || !item || !customPath.trim()) return;

		loading = true;
		try {
			let response;

			switch (item.type) {
				case 'bangumi': {
					response = await api.insertBangumi({
						url: `ss${item.season_id}`,
						whole_season: true,
						path: customPath.trim()
					});
					break;
				}
				case 'favorite': {
					const request: InsertFavoriteRequest = {
						fid: item.fid,
						path: customPath.trim()
					};
					response = await api.insertFavorite(request);
					break;
				}
				case 'collection': {
					const request: InsertCollectionRequest = {
						sid: item.sid,
						mid: item.mid,
						path: customPath.trim()
					};
					response = await api.insertCollection(request);
					break;
				}
				case 'upper': {
					const request: InsertSubmissionRequest = {
						upper_id: item.mid,
						path: customPath.trim()
					};
					response = await api.insertSubmission(request);
					break;
				}
			}

			if (response && response.data) {
				toast.success('订阅成功', {
					description:
						item.type === 'bangumi'
							? `已添加「${getItemTitle()}」，请前往「视频源 → 番剧」启用。`
							: `已订阅${getTypeLabel()}「${getItemTitle()}」到路径「${customPath.trim()}」`
				});
				open = false;
				if (onSuccess) {
					onSuccess();
				}
			}
		} catch (error) {
			console.error(`订阅${getTypeLabel()}失败:`, error);
			toast.error('订阅失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	function handleCancel() {
		open = false;
	}

	$effect(() => {
		if (!open || !pathType || !itemTitle) return;
		let cancelled = false;
		customPath = '';
		pathEdited = false;
		api
			.getDefaultPath(pathType, itemTitle)
			.then((response) => {
				if (!cancelled && !pathEdited) customPath = response.data;
			})
			.catch((error) => {
				if (cancelled) return;
				toast.error('获取默认路径失败，请手动填写', {
					description: (error as ApiError).message
				});
			});
		return () => {
			cancelled = true;
		};
	});

	const typeLabel = $derived(getTypeLabel());
	const itemTitle = $derived(getItemTitle());
</script>

<Sheet bind:open>
	<SheetContent side="right" class="flex w-full flex-col sm:max-w-md">
		<SheetHeader class="px-6 pb-2">
			<SheetTitle class="text-lg">订阅{typeLabel}</SheetTitle>
			<SheetDescription class="text-muted-foreground space-y-1 text-sm">
				<div>即将订阅{typeLabel}「{itemTitle}」</div>
				<div>请手动编辑本地保存路径：</div>
			</SheetDescription>
		</SheetHeader>

		<div class="flex-1 overflow-y-auto px-6">
			<div class="space-y-4 py-4">
				<!-- 项目信息 -->
				<div class="bg-muted/30 rounded-lg border p-4">
					<div class="space-y-2">
						<div class="flex items-center gap-2">
							<span class="text-muted-foreground text-sm font-medium">{typeLabel}名称：</span>
							<span class="text-sm">{itemTitle}</span>
						</div>
						{#if item?.type === 'bangumi'}
							<p class="text-muted-foreground text-sm">下载范围：整季，并持续同步新集。</p>
							<p class="text-muted-foreground text-sm">
								{item.media_count === null ? '总集数待定' : `共 ${item.media_count} 集`}
							</p>
							<p class="text-muted-foreground text-sm">添加后，请前往「视频源 → 番剧」启用。</p>
						{:else if item && item.type !== 'upper'}
							<div class="flex items-center gap-2">
								<span class="text-muted-foreground text-sm font-medium">视频数量：</span>
								<span class="text-sm">{item.media_count} 条</span>
							</div>
						{:else if item?.type === 'upper' && item.sign}
							<div class="flex items-start gap-2">
								<span class="text-muted-foreground text-sm font-medium">个人简介：</span>
								<span class="text-muted-foreground text-sm">{item.sign}</span>
							</div>
						{/if}
					</div>
				</div>

				<!-- 路径输入 -->
				<div class="space-y-3">
					<Label for="custom-path" class="text-sm font-medium">
						本地保存路径 <span class="text-destructive">*</span>
					</Label>
					<Input
						id="custom-path"
						type="text"
						placeholder="请输入保存路径，例如：/home/我的收藏"
						bind:value={customPath}
						oninput={() => (pathEdited = true)}
						disabled={loading}
						class="w-full"
					/>
					<div class="text-muted-foreground space-y-3 text-xs">
						<p>路径将作为文件夹名称，用于存放下载的视频文件。</p>
						<div>
							<p class="mb-2 font-medium">路径示例：</p>
							<div class="space-y-1 pl-4">
								<div class="font-mono text-xs">Mac/Linux: /home/downloads/我的收藏</div>
								<div class="font-mono text-xs">Windows: C:\Downloads\我的收藏</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>

		<SheetFooter class="bg-background flex gap-2 border-t px-6 pt-4">
			<Button
				variant="outline"
				onclick={handleCancel}
				disabled={loading}
				class="flex-1 cursor-pointer"
			>
				取消
			</Button>
			<Button
				onclick={handleSubscribe}
				disabled={loading || !customPath.trim()}
				class="flex-1 cursor-pointer"
			>
				{loading ? '订阅中...' : '确认订阅'}
			</Button>
		</SheetFooter>
	</SheetContent>
</Sheet>
