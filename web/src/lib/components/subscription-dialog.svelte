<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import {
		Sheet,
		SheetContent,
		SheetDescription,
		SheetFooter,
		SheetHeader,
		SheetTitle
	} from '$lib/components/ui/sheet/index.js';
	import { toast } from 'svelte-sonner';
	import api from '$lib/api';
	import type {
		FavoriteWithSubscriptionStatus,
		CollectionWithSubscriptionStatus,
		UpperWithSubscriptionStatus,
		UpsertFavoriteRequest,
		UpsertCollectionRequest,
		UpsertSubmissionRequest,
		ApiError
	} from '$lib/types';

	export let open = false;
	export let item:
		| FavoriteWithSubscriptionStatus
		| CollectionWithSubscriptionStatus
		| UpperWithSubscriptionStatus
		| null = null;
	export let type: 'favorite' | 'collection' | 'upper' = 'favorite';
	export let onSuccess: (() => void) | null = null;

	let customPath = '';
	let loading = false;

	// 根据类型和item生成默认路径
	function generateDefaultPath(): string {
		if (!item) return '';

		switch (type) {
			case 'favorite': {
				const favorite = item as FavoriteWithSubscriptionStatus;
				return `收藏夹/${favorite.title}`;
			}
			case 'collection': {
				const collection = item as CollectionWithSubscriptionStatus;
				return `合集/${collection.title}`;
			}
			case 'upper': {
				const upper = item as UpperWithSubscriptionStatus;
				return `UP主/${upper.uname}`;
			}
			default:
				return '';
		}
	}

	function getTypeLabel(): string {
		switch (type) {
			case 'favorite':
				return '收藏夹';
			case 'collection':
				return '合集';
			case 'upper':
				return 'UP主';
			default:
				return '';
		}
	}

	function getItemTitle(): string {
		if (!item) return '';

		switch (type) {
			case 'favorite':
				return (item as FavoriteWithSubscriptionStatus).title;
			case 'collection':
				return (item as CollectionWithSubscriptionStatus).title;
			case 'upper':
				return (item as UpperWithSubscriptionStatus).uname;
			default:
				return '';
		}
	}

	async function handleSubscribe() {
		if (!item || !customPath.trim()) return;

		loading = true;
		try {
			let response;

			switch (type) {
				case 'favorite': {
					const favorite = item as FavoriteWithSubscriptionStatus;
					const request: UpsertFavoriteRequest = {
						fid: favorite.fid,
						path: customPath.trim()
					};
					response = await api.upsertFavorite(request);
					break;
				}
				case 'collection': {
					const collection = item as CollectionWithSubscriptionStatus;
					const request: UpsertCollectionRequest = {
						sid: collection.sid,
						mid: collection.mid,
						path: customPath.trim()
					};
					response = await api.upsertCollection(request);
					break;
				}
				case 'upper': {
					const upper = item as UpperWithSubscriptionStatus;
					const request: UpsertSubmissionRequest = {
						upper_id: upper.mid,
						path: customPath.trim()
					};
					response = await api.upsertSubmission(request);
					break;
				}
			}

			if (response && response.data) {
				toast.success('订阅成功', {
					description: `已订阅${getTypeLabel()}「${getItemTitle()}」到路径「${customPath.trim()}」`
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

	// 当对话框打开时重置path
	$: if (open && item) {
		customPath = generateDefaultPath();
	}

	const typeLabel = getTypeLabel();
	const itemTitle = getItemTitle();
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
						{#if type === 'favorite'}
							{@const favorite = item as FavoriteWithSubscriptionStatus}
							<div class="flex items-center gap-2">
								<span class="text-muted-foreground text-sm font-medium">视频数量：</span>
								<span class="text-sm">{favorite.media_count} 个</span>
							</div>
						{/if}
						{#if type === 'upper'}
							{@const upper = item as UpperWithSubscriptionStatus}
							{#if upper.sign}
								<div class="flex items-start gap-2">
									<span class="text-muted-foreground text-sm font-medium">个人简介：</span>
									<span class="text-muted-foreground text-sm">{upper.sign}</span>
								</div>
							{/if}
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
