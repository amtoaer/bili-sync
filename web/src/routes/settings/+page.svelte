<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import api from '$lib/api';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { goto } from '$app/navigation';
	import { appStateStore, ToQuery } from '$lib/stores/filter';
	import type { Config, ApiError } from '$lib/types';

	let apiToken = '';
	let config: Config | null = null;
	let formData: Config | null = null;
	let saving = false;
	let loading = false;

	async function loadConfig() {
		loading = true;
		try {
			const response = await api.getConfig();
			config = response.data;
			formData = { ...config }; // 创建配置的副本用于表单绑定
		} catch (error) {
			console.error('加载配置失败:', error);
			toast.error('加载配置失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
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

	async function saveConfig() {
		if (!formData) {
			toast.error('配置未加载');
			return;
		}

		// 基本验证
		if (
			!formData.credential.sessdata.trim() ||
			!formData.credential.bili_jct.trim() ||
			!formData.credential.buvid3.trim() ||
			!formData.credential.dedeuserid.trim() ||
			!formData.credential.ac_time_value.trim()
		) {
			toast.error('请填写完整的B站认证信息');
			return;
		}

		if (!formData.video_name.trim()) {
			toast.error('请设置视频名称模板');
			return;
		}

		if (!formData.page_name.trim()) {
			toast.error('请设置分页名称模板');
			return;
		}

		if (!formData.upper_path.trim()) {
			toast.error('请设置UP主头像保存路径');
			return;
		}

		if (formData.concurrent_limit.video <= 0 || formData.concurrent_limit.page <= 0) {
			toast.error('并发限制必须大于0');
			return;
		}

		saving = true;
		try {
			await api.updateConfig(formData);
			config = { ...formData }; // 更新config以反映保存的状态
			toast.success('配置已保存');
		} catch (error) {
			console.error('保存配置失败:', error);
			toast.error('保存配置失败', {
				description: (error as ApiError).message
			});
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

		loadConfig();
	});
</script>

<svelte:head>
	<title>设置 - Bili Sync</title>
</svelte:head>

<div class="max-w-4xl">
	<div class="space-y-6">
		<!-- API Token 配置 -->
		<Card.Root>
			<Card.Header>
				<Card.Title>API Token 配置</Card.Title>
				<Card.Description>用于身份验证的API令牌</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-4">
				<div class="space-y-2">
					<Label for="api-token">API Token</Label>
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
			</Card.Content>
		</Card.Root>

		<!-- 应用配置 -->
		{#if loading}
			<Card.Root>
				<Card.Content class="py-12 text-center">
					<p class="text-muted-foreground">加载配置中...</p>
				</Card.Content>
			</Card.Root>
		{:else if formData}
			<Card.Root>
				<Card.Header>
					<Card.Title>应用配置</Card.Title>
					<Card.Description>配置Bili Sync的运行参数</Card.Description>
				</Card.Header>
				<Card.Content>
					<Tabs.Root value="basic" class="w-full">
						<Tabs.List class="grid w-full grid-cols-5">
							<Tabs.Trigger value="basic">基本设置</Tabs.Trigger>
							<Tabs.Trigger value="credential">认证信息</Tabs.Trigger>
							<Tabs.Trigger value="filter">视频质量</Tabs.Trigger>
							<Tabs.Trigger value="danmaku">弹幕渲染</Tabs.Trigger>
							<Tabs.Trigger value="advanced">高级设置</Tabs.Trigger>
						</Tabs.List>

						<!-- 基本设置 -->
						<Tabs.Content value="basic" class="mt-6 space-y-6">
							<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
								<div class="space-y-2">
									<Label for="bind-address">绑定地址</Label>
									<Input
										id="bind-address"
										placeholder="127.0.0.1:9999"
										bind:value={formData.bind_address}
									/>
								</div>
								<div class="space-y-2">
									<Label for="interval">同步间隔（秒）</Label>
									<Input id="interval" type="number" min="60" bind:value={formData.interval} />
								</div>
								<div class="space-y-2">
									<Label for="video-name">视频名称模板</Label>
									<Input
										id="video-name"
										placeholder="{'{'}title{'}'}"
										bind:value={formData.video_name}
									/>
								</div>
								<div class="space-y-2">
									<Label for="page-name">分页名称模板</Label>
									<Input
										id="page-name"
										placeholder="{'{'}bvid{'}'}"
										bind:value={formData.page_name}
									/>
								</div>
								<div class="space-y-2">
									<Label for="upper-path">UP主头像保存路径</Label>
									<Input
										id="upper-path"
										placeholder="/path/to/upper/faces"
										bind:value={formData.upper_path}
									/>
								</div>
								<div class="space-y-2">
									<Label for="time-format">时间格式</Label>
									<Input
										id="time-format"
										placeholder="%Y-%m-%d %H:%M:%S"
										bind:value={formData.time_format}
									/>
								</div>
							</div>

							<Separator />

							<div class="space-y-4">
								<div class="flex items-center space-x-2">
									<Switch id="cdn-sorting" bind:checked={formData.cdn_sorting} />
									<Label for="cdn-sorting">启用CDN排序</Label>
								</div>
							</div>
						</Tabs.Content>

						<!-- 认证信息 -->
						<Tabs.Content value="credential" class="mt-6 space-y-6">
							<div class="space-y-4">
								<div class="space-y-2">
									<Label for="sessdata">SESSDATA</Label>
									<Input
										id="sessdata"
										type="password"
										placeholder="请输入SESSDATA"
										bind:value={formData.credential.sessdata}
									/>
								</div>
								<div class="space-y-2">
									<Label for="bili-jct">bili_jct</Label>
									<Input
										id="bili-jct"
										type="password"
										placeholder="请输入bili_jct"
										bind:value={formData.credential.bili_jct}
									/>
								</div>
								<div class="space-y-2">
									<Label for="buvid3">buvid3</Label>
									<Input
										id="buvid3"
										placeholder="请输入buvid3"
										bind:value={formData.credential.buvid3}
									/>
								</div>
								<div class="space-y-2">
									<Label for="dedeuserid">dedeuserid</Label>
									<Input
										id="dedeuserid"
										placeholder="请输入dedeuserid"
										bind:value={formData.credential.dedeuserid}
									/>
								</div>
								<div class="space-y-2">
									<Label for="ac-time-value">ac_time_value</Label>
									<Input
										id="ac-time-value"
										placeholder="请输入ac_time_value"
										bind:value={formData.credential.ac_time_value}
									/>
								</div>
							</div>
						</Tabs.Content>

						<!-- 过滤规则 -->
						<Tabs.Content value="filter" class="mt-6 space-y-6">
							<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
								<div class="space-y-2">
									<Label for="video-max-quality">最高视频质量</Label>
									<select
										id="video-max-quality"
										class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
										bind:value={formData.filter_option.video_max_quality}
									>
										<option value="Quality360p">360p</option>
										<option value="Quality480p">480p</option>
										<option value="Quality720p">720p</option>
										<option value="Quality1080p">1080p</option>
										<option value="Quality1080pPLUS">1080p+</option>
										<option value="Quality1080p60">1080p60</option>
										<option value="Quality4k">4K</option>
										<option value="QualityHdr">HDR</option>
										<option value="QualityDolby">杜比视界</option>
										<option value="Quality8k">8K</option>
									</select>
								</div>
								<div class="space-y-2">
									<Label for="video-min-quality">最低视频质量</Label>
									<select
										id="video-min-quality"
										class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
										bind:value={formData.filter_option.video_min_quality}
									>
										<option value="Quality360p">360p</option>
										<option value="Quality480p">480p</option>
										<option value="Quality720p">720p</option>
										<option value="Quality1080p">1080p</option>
										<option value="Quality1080pPLUS">1080p+</option>
										<option value="Quality1080p60">1080p60</option>
										<option value="Quality4k">4K</option>
										<option value="QualityHdr">HDR</option>
										<option value="QualityDolby">杜比视界</option>
										<option value="Quality8k">8K</option>
									</select>
								</div>
								<div class="space-y-2">
									<Label for="audio-max-quality">最高音频质量</Label>
									<select
										id="audio-max-quality"
										class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
										bind:value={formData.filter_option.audio_max_quality}
									>
										<option value="Quality64k">64k</option>
										<option value="Quality132k">132k</option>
										<option value="Quality192k">192k</option>
										<option value="QualityDolby">杜比全景声</option>
										<option value="QualityHiRES">Hi-RES</option>
									</select>
								</div>
								<div class="space-y-2">
									<Label for="audio-min-quality">最低音频质量</Label>
									<select
										id="audio-min-quality"
										class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
										bind:value={formData.filter_option.audio_min_quality}
									>
										<option value="Quality64k">64k</option>
										<option value="Quality132k">132k</option>
										<option value="Quality192k">192k</option>
										<option value="QualityDolby">杜比全景声</option>
										<option value="QualityHiRES">Hi-RES</option>
									</select>
								</div>
							</div>

							<Separator />

							<div class="space-y-4">
								<Label>视频编码格式偏好（按优先级排序）</Label>
								<p class="text-muted-foreground text-sm">排在前面的编码格式优先级更高</p>
								<div class="space-y-2">
									{#each formData.filter_option.codecs as codec, index}
										<div class="flex items-center space-x-2 rounded border p-2">
											<span class="w-8 text-sm font-medium">{index + 1}.</span>
											<span class="flex-1">{codec}</span>
											<div class="flex space-x-1">
												<button
													type="button"
													class="hover:bg-muted rounded border px-2 py-1 text-xs disabled:opacity-50"
													disabled={index === 0}
													onclick={() => {
														if (formData && index > 0) {
															const newCodecs = [...formData.filter_option.codecs];
															[newCodecs[index - 1], newCodecs[index]] = [
																newCodecs[index],
																newCodecs[index - 1]
															];
															formData.filter_option.codecs = newCodecs;
														}
													}}
												>
													↑
												</button>
												<button
													type="button"
													class="hover:bg-muted rounded border px-2 py-1 text-xs disabled:opacity-50"
													disabled={index === formData.filter_option.codecs.length - 1}
													onclick={() => {
														if (formData && index < formData.filter_option.codecs.length - 1) {
															const newCodecs = [...formData.filter_option.codecs];
															[newCodecs[index], newCodecs[index + 1]] = [
																newCodecs[index + 1],
																newCodecs[index]
															];
															formData.filter_option.codecs = newCodecs;
														}
													}}
												>
													↓
												</button>
												<button
													type="button"
													class="hover:bg-destructive hover:text-destructive-foreground rounded border px-2 py-1 text-xs"
													onclick={() => {
														if (formData) {
															formData.filter_option.codecs = formData.filter_option.codecs.filter(
																(_, i) => i !== index
															);
														}
													}}
												>
													×
												</button>
											</div>
										</div>
									{/each}

									{#if formData.filter_option.codecs.length < 3}
										<div class="space-y-2">
											<Label>添加编码格式</Label>
											<div class="flex space-x-2">
												{#each ['AV1', 'HEV', 'AVC'] as codec}
													{#if !formData.filter_option.codecs.includes(codec)}
														<button
															type="button"
															class="hover:bg-muted rounded border px-3 py-1 text-sm"
															onclick={() => {
																if (formData) {
																	formData.filter_option.codecs = [
																		...formData.filter_option.codecs,
																		codec
																	];
																}
															}}
														>
															+ {codec}
														</button>
													{/if}
												{/each}
											</div>
										</div>
									{/if}
								</div>
							</div>

							<Separator />

							<div class="space-y-4">
								<div class="flex items-center space-x-2">
									<Switch
										id="no-dolby-video"
										bind:checked={formData.filter_option.no_dolby_video}
									/>
									<Label for="no-dolby-video">排除杜比视界视频</Label>
								</div>
								<div class="flex items-center space-x-2">
									<Switch
										id="no-dolby-audio"
										bind:checked={formData.filter_option.no_dolby_audio}
									/>
									<Label for="no-dolby-audio">排除杜比全景声音频</Label>
								</div>
								<div class="flex items-center space-x-2">
									<Switch id="no-hdr" bind:checked={formData.filter_option.no_hdr} />
									<Label for="no-hdr">排除HDR视频</Label>
								</div>
								<div class="flex items-center space-x-2">
									<Switch id="no-hires" bind:checked={formData.filter_option.no_hires} />
									<Label for="no-hires">排除Hi-RES音频</Label>
								</div>
							</div>
						</Tabs.Content>

						<!-- 弹幕设置 -->
						<Tabs.Content value="danmaku" class="mt-6 space-y-6">
							<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
								<div class="space-y-2">
									<Label for="danmaku-duration">弹幕持续时间（秒）</Label>
									<Input
										id="danmaku-duration"
										type="number"
										min="1"
										step="0.1"
										bind:value={formData.danmaku_option.duration}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-font">字体</Label>
									<Input
										id="danmaku-font"
										placeholder="黑体"
										bind:value={formData.danmaku_option.font}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-font-size">字体大小</Label>
									<Input
										id="danmaku-font-size"
										type="number"
										min="8"
										max="72"
										bind:value={formData.danmaku_option.font_size}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-width-ratio">宽度比例</Label>
									<Input
										id="danmaku-width-ratio"
										type="number"
										min="0.1"
										max="3"
										step="0.1"
										bind:value={formData.danmaku_option.width_ratio}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-horizontal-gap">水平间距</Label>
									<Input
										id="danmaku-horizontal-gap"
										type="number"
										min="0"
										step="1"
										bind:value={formData.danmaku_option.horizontal_gap}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-lane-size">轨道大小</Label>
									<Input
										id="danmaku-lane-size"
										type="number"
										min="8"
										max="128"
										bind:value={formData.danmaku_option.lane_size}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-float-percentage">滚动弹幕高度百分比</Label>
									<Input
										id="danmaku-float-percentage"
										type="number"
										min="0"
										max="1"
										step="0.01"
										bind:value={formData.danmaku_option.float_percentage}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-bottom-percentage">底部弹幕高度百分比</Label>
									<Input
										id="danmaku-bottom-percentage"
										type="number"
										min="0"
										max="1"
										step="0.01"
										bind:value={formData.danmaku_option.bottom_percentage}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-opacity">透明度 (0-255)</Label>
									<Input
										id="danmaku-opacity"
										type="number"
										min="0"
										max="255"
										bind:value={formData.danmaku_option.opacity}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-outline">描边宽度</Label>
									<Input
										id="danmaku-outline"
										type="number"
										min="0"
										max="5"
										step="0.1"
										bind:value={formData.danmaku_option.outline}
									/>
								</div>
								<div class="space-y-2">
									<Label for="danmaku-time-offset">时间偏移（秒）</Label>
									<Input
										id="danmaku-time-offset"
										type="number"
										step="0.1"
										bind:value={formData.danmaku_option.time_offset}
									/>
								</div>
							</div>

							<Separator />

							<div class="space-y-4">
								<div class="flex items-center space-x-2">
									<Switch id="danmaku-bold" bind:checked={formData.danmaku_option.bold} />
									<Label for="danmaku-bold">粗体显示</Label>
								</div>
							</div>
						</Tabs.Content>

						<!-- 高级设置 -->
						<Tabs.Content value="advanced" class="mt-6 space-y-6">
							<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
								<div class="space-y-2">
									<Label for="video-concurrent">视频并发数</Label>
									<Input
										id="video-concurrent"
										type="number"
										min="1"
										max="20"
										bind:value={formData.concurrent_limit.video}
									/>
								</div>
								<div class="space-y-2">
									<Label for="page-concurrent">分页并发数</Label>
									<Input
										id="page-concurrent"
										type="number"
										min="1"
										max="20"
										bind:value={formData.concurrent_limit.page}
									/>
								</div>
								<div class="space-y-2">
									<Label for="nfo-time-type">NFO时间类型</Label>
									<select
										id="nfo-time-type"
										class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
										bind:value={formData.nfo_time_type}
									>
										<option value="FavTime">收藏时间</option>
										<option value="PubTime">发布时间</option>
									</select>
								</div>
							</div>
						</Tabs.Content>
					</Tabs.Root>

					<div class="mt-6 flex justify-end">
						<Button onclick={saveConfig} disabled={saving}>
							{saving ? '保存中...' : '保存配置'}
						</Button>
					</div>
				</Card.Content>
			</Card.Root>
		{/if}
	</div>
</div>
