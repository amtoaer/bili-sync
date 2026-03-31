<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import {
		Sheet,
		SheetContent,
		SheetDescription,
		SheetHeader,
		SheetTitle
	} from '$lib/components/ui/sheet/index.js';
	import { toast } from 'svelte-sonner';
	import api from '$lib/api';
	import type { ApiError, YoutubePlaylist } from '$lib/types';

	interface Props {
		open: boolean;
		item: YoutubePlaylist | null;
		onSuccess: (() => void) | null;
	}

	let { open = $bindable(false), item = null, onSuccess = null }: Props = $props();

	let customPath = $state('');
	let loading = $state(false);

	async function generateDefaultPath() {
		if (!item) return '';
		const response = await api.getYoutubeDefaultPath(item.name);
		return response.data;
	}

	async function handleSubmit() {
		if (!item || !customPath.trim()) {
			toast.error('请输入本地保存路径');
			return;
		}
		loading = true;
		try {
			await api.insertYoutubePlaylist({
				playlistId: item.playlistId,
				name: item.name,
				url: item.url,
				thumbnail: item.thumbnail ?? null,
				path: customPath.trim()
			});
			toast.success('YouTube 播放列表添加成功', {
				description: `已添加播放列表「${item.name}」`
			});
			open = false;
			onSuccess?.();
		} catch (error) {
			console.error('添加 YouTube 播放列表失败:', error);
			toast.error('添加失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (open && item) {
			generateDefaultPath()
				.then((path) => {
					customPath = path;
				})
				.catch((error) => {
					toast.error('获取默认路径失败', {
						description: (error as ApiError).message
					});
					customPath = '';
				});
		}
	});
</script>

<Sheet bind:open>
	<SheetContent side="right" class="flex w-full flex-col sm:max-w-md">
		<SheetHeader class="px-6 pb-2">
			<SheetTitle class="text-lg">添加 YouTube 播放列表</SheetTitle>
			<SheetDescription class="text-muted-foreground text-sm">
				为播放列表「{item?.name || ''}」设置本地保存路径。
			</SheetDescription>
		</SheetHeader>

		<div class="flex-1 overflow-y-auto px-6">
			<div class="space-y-4 py-4">
				<div class="bg-muted/30 rounded-lg border p-4 text-sm">
					<div class="font-medium">{item?.name}</div>
					{#if item?.ownerName}
						<div class="text-muted-foreground mt-1">创建者：{item.ownerName}</div>
					{/if}
					{#if item?.videoCount}
						<div class="text-muted-foreground mt-1">视频数量：{item.videoCount}</div>
					{/if}
				</div>

				<div class="space-y-3">
					<Label for="youtube-playlist-custom-path" class="text-sm font-medium">
						本地保存路径 <span class="text-destructive">*</span>
					</Label>
					<Input
						id="youtube-playlist-custom-path"
						type="text"
						placeholder="请输入保存路径"
						bind:value={customPath}
						disabled={loading}
						class="w-full"
					/>
					<p class="text-muted-foreground text-xs">
						播放列表中的视频会按当前 YouTube 下载规则保存到这个路径下。
					</p>
				</div>
			</div>
		</div>

		<div class="border-t px-6 py-4">
			<div class="flex gap-3">
				<Button variant="outline" class="flex-1" onclick={() => (open = false)} disabled={loading}>
					取消
				</Button>
				<Button class="flex-1" onclick={handleSubmit} disabled={loading || !customPath.trim()}>
					{loading ? '添加中...' : '添加到视频源'}
				</Button>
			</div>
		</div>
	</SheetContent>
</Sheet>
