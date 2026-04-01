<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import type { ApiError } from '$lib/types';

	let url = '';
	let path = '';
	let submitting = false;

	async function handleSubmit() {
		if (!url.trim()) {
			toast.error('请先输入 YouTube 链接');
			return;
		}
		submitting = true;
		try {
			const response = await api.manualSubmitYoutubeLink({
				url: url.trim(),
				path: path.trim() || null
			});
			toast.success('YouTube 链接已提交', {
				description: response.data.queued
					? '链接已经进入后台处理队列，页面不再等待解析完成。'
					: '链接已提交，后台处理中。'
			});
			url = '';
			path = '';
		} catch (error) {
			console.error('提交 YouTube 链接失败：', error);
			toast.error('提交失败', {
				description: (error as ApiError).message
			});
		} finally {
			submitting = false;
		}
	}

	onMount(() => {
		setBreadcrumb([{ label: 'YouTube 手动提交链接' }]);
	});
</script>

<svelte:head>
	<title>YouTube 手动提交链接 - Bili Sync</title>
</svelte:head>

<div class="space-y-6">
	<div class="bg-card rounded-xl border p-6">
		<div class="space-y-2">
			<h2 class="text-xl font-semibold">手动提交 YouTube 链接</h2>
			<p class="text-muted-foreground text-sm">
				支持频道链接、播放列表链接和单个视频链接。提交后会立即返回，解析与入库会在后台继续处理。
			</p>
		</div>

		<div class="mt-6 space-y-5">
			<div class="space-y-2">
				<Label for="youtube-manual-url">YouTube 链接</Label>
				<Input
					id="youtube-manual-url"
					type="text"
					bind:value={url}
					placeholder="https://www.youtube.com/playlist?list=... / https://www.youtube.com/@channel / https://youtu.be/..."
				/>
			</div>

			<div class="space-y-2">
				<Label for="youtube-manual-path">保存路径（可选）</Label>
				<Input
					id="youtube-manual-path"
					type="text"
					bind:value={path}
					placeholder="留空时，频道/播放列表使用默认模板，单视频在容器中默认保存到 /download"
				/>
				<p class="text-muted-foreground text-xs">
					如果提交的是单个视频并且你想自定义目录，这里请填写绝对路径。
				</p>
			</div>
		</div>

		<div class="mt-8 flex justify-end">
			<Button onclick={handleSubmit} disabled={submitting || !url.trim()}>
				{submitting ? '提交中...' : '提交链接'}
			</Button>
		</div>
	</div>

	<div class="bg-card rounded-xl border p-6">
		<div class="space-y-3">
			<h3 class="font-semibold">处理规则</h3>
			<div class="text-muted-foreground space-y-2 text-sm">
				<p>频道链接：后台解析完成后自动加入 YouTube 视频源。</p>
				<p>播放列表链接：后台解析完成后自动加入 YouTube 视频源。</p>
				<p>单视频链接：后台解析完成后自动创建一条 YouTube 手动下载任务，不影响现有 B 站流程。</p>
			</div>
		</div>
	</div>
</div>
