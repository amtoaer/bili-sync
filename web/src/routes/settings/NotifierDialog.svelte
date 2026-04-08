<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { toast } from 'svelte-sonner';
	import type { Notifier } from '$lib/types';

	export let notifier: Notifier | null = null;
	export let onSave: (notifier: Notifier) => void;
	export let onCancel: () => void;

	let type: 'telegram' | 'webhook' = 'telegram';
	let botToken = '';
	let chatId = '';
	let skipImage = false;
	let webhookUrl = '';
	let webhookTemplate = '';
	let webhookHeaders: { key: string; value: string }[] = [];

	// 初始化表单
	$: {
		if (notifier) {
			if (notifier.type === 'telegram') {
				type = 'telegram';
				botToken = notifier.bot_token;
				chatId = notifier.chat_id;
				skipImage = notifier.skip_image;
			} else {
				type = 'webhook';
				webhookUrl = notifier.url;
				webhookTemplate = notifier.template || '';
				if (notifier.headers) {
					webhookHeaders = Object.entries(notifier.headers).map(([key, value]) => ({ key, value }));
				} else {
					webhookHeaders = [];
				}
			}
		} else {
			type = 'telegram';
			botToken = '';
			chatId = '';
			skipImage = false;
			webhookUrl = '';
			webhookTemplate = '';
			webhookHeaders = [];
		}
	}

	function handleSave() {
		if (type === 'telegram') {
			if (!botToken.trim()) {
				toast.error('请输入 Bot Token');
				return;
			}
			if (!chatId.trim()) {
				toast.error('请输入 Chat ID');
				return;
			}

			const newNotifier: Notifier = {
				type: 'telegram',
				bot_token: botToken.trim(),
				chat_id: chatId.trim(),
				skip_image: skipImage
			};
			onSave(newNotifier);
		} else {
			if (!webhookUrl.trim()) {
				toast.error('请输入 Webhook URL');
				return;
			}

			try {
				new URL(webhookUrl.trim());
			} catch {
				toast.error('请输入有效的 Webhook URL');
				return;
			}

			const headers: Record<string, string> = {};
			for (const { key, value } of webhookHeaders) {
				const trimmedKey = key.trim();
				const trimmedValue = value.trim();
				if (trimmedKey && trimmedValue) {
					headers[trimmedKey] = trimmedValue;
				}
			}

			const newNotifier: Notifier = {
				type: 'webhook',
				url: webhookUrl.trim(),
				template: webhookTemplate.trim() || null,
				headers: Object.keys(headers).length > 0 ? headers : null
			};
			onSave(newNotifier);
		}
	}
</script>

<div class="space-y-4 py-4">
	<div class="space-y-2">
		<Label for="notifier-type">通知器类型</Label>
		<select
			id="notifier-type"
			class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
			bind:value={type}
		>
			<option value="telegram">Telegram Bot</option>
			<option value="webhook">Webhook</option>
		</select>
	</div>

	{#if type === 'telegram'}
		<div class="space-y-2">
			<Label for="bot-token">Bot Token</Label>
			<Input
				id="bot-token"
				placeholder="1234567890:ABCdefGHIjklMNOpqrsTUVwxyz"
				bind:value={botToken}
			/>
			<p class="text-muted-foreground text-xs">从 @BotFather 获取的 Bot Token</p>
		</div>
		<div class="space-y-2">
			<Label for="chat-id">Chat ID</Label>
			<Input id="chat-id" placeholder="-1001234567890" bind:value={chatId} />
			<p class="text-muted-foreground text-xs">目标聊天室的 ID（个人用户、群组或频道）</p>
		</div>
		<div class="flex items-center gap-2">
			<Checkbox id="skip-image" bind:checked={skipImage} />
			<Label for="skip-image" class="text-sm font-normal">仅发送文字</Label>
		</div>
	{:else if type === 'webhook'}
		<div class="space-y-2">
			<Label for="webhook-url">Webhook URL</Label>
			<Input id="webhook-url" placeholder="请输入 Webhook 地址" bind:value={webhookUrl} />
		</div>
		<div class="space-y-2">
			<Label for="webhook-template">模板（可选）</Label>
			<textarea
				id="webhook-template"
				class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex min-h-[120px] w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
				placeholder={'{"text": "{{{message}}}"}'}
				bind:value={webhookTemplate}
			></textarea>
			<p class="text-muted-foreground text-xs">
				用于渲染 Webhook 的 Handlebars 模板。如果不填写，将使用默认模板。<br />
				可用变量：<code class="text-xs">message</code>（通知内容）、<code class="text-xs"
					>image_url</code
				>（封面图片地址，无图时为 null）
			</p>
		</div>

		<div class="space-y-2">
			<div class="flex items-center justify-between">
				<Label>自定义请求头（可选）</Label>
				<Button
					variant="ghost"
					size="sm"
					onclick={() => (webhookHeaders = [...webhookHeaders, { key: '', value: '' }])}
				>
					+ 添加请求头
				</Button>
			</div>
			{#each webhookHeaders as header, index (index)}
				<div class="flex items-center gap-2">
					<Input
						placeholder="Header 名称（例如 Authorization）"
						bind:value={header.key}
						class="flex-1"
					/>
					<Input
						placeholder="Header 值"
						bind:value={header.value}
						class="flex-1"
						type={header.key.toLowerCase() === 'authorization' ? 'password' : 'text'}
					/>
					<Button
						variant="ghost"
						size="sm"
						onclick={() => (webhookHeaders = webhookHeaders.filter((_, i) => i !== index))}
						class="h-10 px-2"
					>
						×
					</Button>
				</div>
			{/each}
			<p class="text-muted-foreground text-xs">
				添加自定义请求头，例如：Authorization: Bearer your_token
			</p>
		</div>
	{/if}
</div>

<div class="flex justify-end gap-3">
	<Button variant="outline" onclick={onCancel}>取消</Button>
	<Button onclick={handleSave}>确认</Button>
</div>
