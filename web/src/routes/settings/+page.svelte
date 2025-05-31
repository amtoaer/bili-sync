<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Tabs, TabsContent, TabsList, TabsTrigger } from '$lib/components/ui/tabs/index.js';
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import api from '$lib/api';
	import { toast } from 'svelte-sonner';

	let apiToken = '';
	let saving = false;

	const breadcrumbItems = [
		{ href: '/', label: '主页' },
		{ label: '设置', isActive: true }
	];

	// 处理面包屑中"主页"的点击
	function handleHomeClick() {
		history.back();
	}

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
		// 从localStorage读取已保存的token
		const savedToken = localStorage.getItem('authToken');
		if (savedToken) {
			apiToken = savedToken;
		}
	});
</script>

<svelte:head>
	<title>设置 - Bili Sync</title>
</svelte:head>

<div class="bg-background min-h-screen w-full">
	<div class="w-full px-6 py-6">
		<!-- 面包屑导航 -->
		<div class="mb-6">
			<BreadCrumb
				items={breadcrumbItems.map((item) =>
					item.href === '/' ? { ...item, href: undefined, onClick: handleHomeClick } : item
				)}
			/>
		</div>

		<!-- 设置内容 -->
		<div class="max-w-4xl">
			<Tabs value="general" orientation="vertical" class="flex gap-6">
				<!-- 侧边栏选项卡 -->
				<TabsList class="h-fit w-48 flex-col justify-start p-1">
					<TabsTrigger value="general" class="w-full justify-start">常规</TabsTrigger>
				</TabsList>

				<!-- 设置内容区域 -->
				<div class="flex-1">
					<TabsContent value="general" class="mt-0">
						<Card>
							<CardHeader>
								<CardTitle>常规设置</CardTitle>
							</CardHeader>
							<CardContent class="space-y-6">
								<!-- API Token 配置 -->
								<div class="space-y-2">
									<Label for="api-token">API Token</Label>
									<div class="space-y-2">
										<Input
											id="api-token"
											type="password"
											placeholder="请输入API Token"
											bind:value={apiToken}
											class="max-w-md"
										/>
										<p class="text-muted-foreground text-xs">
											用于身份验证的API令牌，请确保令牌安全性
										</p>
									</div>
									<Button onclick={saveApiToken} disabled={saving} class="mt-2">
										{saving ? '保存中...' : '保存'}
									</Button>
								</div>
							</CardContent>
						</Card>
					</TabsContent>
				</div>
			</Tabs>
		</div>
	</div>
</div>
