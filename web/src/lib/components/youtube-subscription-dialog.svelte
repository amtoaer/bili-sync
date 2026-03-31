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
	import type { ApiError, YoutubeSubscription } from '$lib/types';

	interface Props {
		open: boolean;
		item: YoutubeSubscription | null;
		onSuccess: (() => void) | null;
	}

	let { open = $bindable(false), item = null, onSuccess = null }: Props = $props();

	let customPath = $state('');
	let loading = $state(false);

	async function generateDefaultPath(): Promise<string> {
		if (!item) return '';
		return (await api.getYoutubeDefaultPath(item.name)).data;
	}

	async function handleSubscribe() {
		if (!item || !customPath.trim()) return;

		loading = true;
		try {
			await api.insertYoutubeChannel({
				channelId: item.channelId,
				name: item.name,
				url: item.url,
				thumbnail: item.thumbnail ?? null,
				path: customPath.trim()
			});
			toast.success('YouTube 频道订阅成功', {
				description: `已添加频道「${item.name}」`
			});
			open = false;
			onSuccess?.();
		} catch (error) {
			console.error('订阅 YouTube 频道失败:', error);
			toast.error('订阅失败', {
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
			<SheetTitle class="text-lg">订阅 YouTube 频道</SheetTitle>
			<SheetDescription class="text-muted-foreground text-sm">
				为频道「{item?.name || ''}」设置本地保存路径。
			</SheetDescription>
		</SheetHeader>

		<div class="flex-1 overflow-y-auto px-6">
			<div class="space-y-4 py-4">
				<div class="bg-muted/30 rounded-lg border p-4">
					<div class="space-y-2">
						<div class="text-sm font-medium">{item?.name}</div>
						<div class="text-muted-foreground text-xs break-all">{item?.url}</div>
					</div>
				</div>

				<div class="space-y-3">
					<Label for="youtube-custom-path" class="text-sm font-medium">
						本地保存路径 <span class="text-destructive">*</span>
					</Label>
					<Input
						id="youtube-custom-path"
						type="text"
						placeholder="请输入保存路径"
						bind:value={customPath}
						disabled={loading}
						class="w-full"
					/>
					<p class="text-muted-foreground text-xs">
						下载时会在这个路径下按视频标题生成目录和文件。
					</p>
				</div>
			</div>
		</div>

		<SheetFooter class="bg-background flex gap-2 border-t px-6 pt-4">
			<Button variant="outline" onclick={() => (open = false)} disabled={loading} class="flex-1">
				取消
			</Button>
			<Button onclick={handleSubscribe} disabled={loading || !customPath.trim()} class="flex-1">
				{loading ? '订阅中...' : '确认订阅'}
			</Button>
		</SheetFooter>
	</SheetContent>
</Sheet>
