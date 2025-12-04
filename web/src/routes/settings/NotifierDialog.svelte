<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { toast } from 'svelte-sonner';
	import type { Notifier } from '$lib/types';

	const jsonExample = '{"text": "您的消息内容"}';

	export let notifier: Notifier | null = null;
	export let onSave: (notifier: Notifier) => void;
	export let onCancel: () => void;

	let type: 'telegram' | 'webhook' = 'telegram';
	let botToken = '';
	let chatId = '';
	let webhookUrl = '';
	let webhookTemplate = '';

	// 初始化表单
	$: {
		if (notifier) {
			if (notifier.type === 'telegram') {
				type = 'telegram';
				botToken = notifier.bot_token;
				chatId = notifier.chat_id;
			} else {
				type = 'webhook';
				webhookUrl = notifier.url;
				webhookTemplate = notifier.template || '';
			}
		} else {
			type = 'telegram';
			botToken = '';
			chatId = '';
			webhookUrl = '';
			webhookTemplate = '';
		}
	}

	function handleSave() {
		// 验证表单
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
				chat_id: chatId.trim()
			};
			onSave(newNotifier);
		} else {
			if (!webhookUrl.trim()) {
				toast.error('请输入 Webhook URL');
				return;
			}

			// 简单的 URL 验证
			try {
				new URL(webhookUrl.trim());
			} catch {
				toast.error('请输入有效的 Webhook URL');
				return;
			}

			const newNotifier: Notifier = {
				type: 'webhook',
				url: webhookUrl.trim(),
				template: webhookTemplate.trim() || null
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
	{:else if type === 'webhook'}
		<div class="space-y-2">
			<Label for="webhook-url">Webhook URL</Label>
			<Input id="webhook-url" placeholder="https://example.com/webhook" bind:value={webhookUrl} />
			<p class="text-muted-foreground text-xs">
				接收通知的 Webhook 地址<br />
				格式示例：{jsonExample}
			</p>
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
				可用变量：<code class="text-xs">message</code>（通知内容）
			</p>
		</div>
	{/if}
</div>

<div class="flex justify-end gap-3">
	<Button variant="outline" onclick={onCancel}>取消</Button>
	<Button onclick={handleSave}>确认</Button>
</div>
