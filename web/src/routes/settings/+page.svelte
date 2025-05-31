<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import api from '$lib/api';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { goto } from '$app/navigation';
	import { appStateStore, ToQuery } from '$lib/stores/filter';

	let apiToken = '';
	let saving = false;

	async function saveApiToken() {
		if (!apiToken.trim()) {
			toast.error('请输入有效的API Token');
			return;
		}

		saving = true;
		try {
			api.setAuthToken(apiToken.trim());
			toast.success('API Token 已保存');
		} catch (error) {
			console.error('保存API Token失败:', error);
			toast.error('保存失败，请重试');
		} finally {
			saving = false;
		}
	}

	onMount(() => {
		setBreadcrumb([
			{
				label: '主页',
				onClick: () => {
					goto(`/${ToQuery($appStateStore)}`);
				}
			},
			{ label: '设置', isActive: true }
		]);
		const savedToken = localStorage.getItem('authToken');
		if (savedToken) {
			apiToken = savedToken;
		}
	});
</script>

<svelte:head>
	<title>设置 - Bili Sync</title>
</svelte:head>

<div class="max-w-4xl">
	<div class="space-y-8">
		<!-- API Token 配置 -->
		<div class="border-border border-b pb-6">
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
				<div class="lg:col-span-1">
					<Label class="text-base font-semibold">API Token</Label>
					<p class="text-muted-foreground mt-1 text-sm">用于身份验证的API令牌</p>
				</div>
				<div class="space-y-4 lg:col-span-2">
					<div class="space-y-2">
						<Input
							id="api-token"
							type="password"
							placeholder="请输入API Token"
							bind:value={apiToken}
							class="max-w-lg"
						/>
						<p class="text-muted-foreground text-xs">请确保令牌的安全性，不要与他人分享</p>
					</div>
					<Button onclick={saveApiToken} disabled={saving} size="sm">
						{saving ? '保存中...' : '保存'}
					</Button>
				</div>
			</div>
		</div>
	</div>
</div>
