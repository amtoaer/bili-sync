<script lang="ts">
	import HeartIcon from '@lucide/svelte/icons/heart';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import UserIcon from '@lucide/svelte/icons/user';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { useSidebar } from '$lib/components/ui/sidebar/context.svelte.js';
	import { page } from '$app/state';
	import { appStateStore, setVideoSourceFilter, clearAll } from '$lib/stores/filter';

	import { onMount } from 'svelte';
	import { type VideoSourcesResponse } from '$lib/types';
	import { VIDEO_SOURCES } from '$lib/consts';
	import api from '$lib/api';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import { goto } from '$app/navigation';

	const sidebar = useSidebar();
	const search_params = $derived(page.url.searchParams);

	let video_sources: VideoSourcesResponse | null = $state.raw(null);

	onMount(async () => {
		try {
			video_sources = (await api.getVideoSources()).data;
		} catch (error) {
			console.error('加载视频来源失败:', error);
		}
	});

	const items = Object.values(VIDEO_SOURCES);

	function handleSourceClick(sourceType: string, sourceId: number) {
		// 更新全局视频源筛选状态
		setVideoSourceFilter(sourceType, sourceId.toString());

		const params = new URLSearchParams();

		// 保持当前的搜索查询
		const currentState = $appStateStore;
		if (currentState.query.trim()) {
			params.set('query', currentState.query);
		}

		// 清除其他视频源参数，设置新的筛选参数
		for (const videoSource of Object.values(VIDEO_SOURCES)) {
			params.delete(videoSource.type);
		}
		params.delete('page');
		params.set(sourceType, sourceId.toString());

		goto(`/?${params.toString()}`);

		if (sidebar.isMobile) {
			sidebar.setOpenMobile(false);
		}
	}

	function handleLogoClick() {
		clearAll();
		goto('/');

		if (sidebar.isMobile) {
			sidebar.setOpenMobile(false);
		}
	}
</script>

<Sidebar.Root class="border-border bg-background border-r">
	<Sidebar.Header class="border-border flex h-[73px] items-center border-b">
		<a
			href="/"
			class="flex w-full items-center gap-3 px-4 py-3 hover:cursor-pointer"
			onclick={handleLogoClick}
		>
			<div class="flex h-8 w-8 items-center justify-center overflow-hidden rounded-lg">
				<img src="/favicon.png" alt="Bili Sync" class="h-6 w-6" />
			</div>
			<div class="grid flex-1 text-left text-sm leading-tight">
				<span class="truncate font-semibold">Bili Sync</span>
				<span class="text-muted-foreground truncate text-xs">视频管理系统</span>
			</div>
		</a>
	</Sidebar.Header>
	<Sidebar.Content class="flex flex-col px-2 py-3">
		<div class="flex-1">
			<Sidebar.Group>
				<Sidebar.GroupLabel
					class="text-muted-foreground mb-2 px-2 text-xs font-medium uppercase tracking-wider"
				>
					视频来源
				</Sidebar.GroupLabel>
				<Sidebar.GroupContent>
					<Sidebar.Menu class="space-y-1">
						{#each items as item (item.type)}
							<Collapsible.Root class="group/collapsible">
								<Sidebar.MenuItem>
									<Collapsible.Trigger class="w-full">
										{#snippet child({ props })}
											<Sidebar.MenuButton
												{...props}
												class="hover:bg-accent/50 text-foreground flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 font-medium transition-all duration-200"
											>
												<div class="flex flex-1 items-center gap-3">
													<item.icon class="text-muted-foreground h-4 w-4" />
													<span class="text-sm">{item.title}</span>
												</div>
												<ChevronRightIcon
													class="text-muted-foreground h-3 w-3 transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90"
												/>
											</Sidebar.MenuButton>
										{/snippet}
									</Collapsible.Trigger>
									<Collapsible.Content class="mt-1">
										<div class="border-border ml-5 space-y-0.5 border-l pl-2">
											{#if video_sources}
												{#if video_sources[item.type as keyof VideoSourcesResponse]?.length > 0}
													{#each video_sources[item.type as keyof VideoSourcesResponse] as source (source.id)}
														<Sidebar.MenuItem>
															<button
																class="text-foreground hover:bg-accent/50 w-full cursor-pointer rounded-md px-3 py-2 text-left text-sm transition-all duration-200"
																onclick={() => handleSourceClick(item.type, source.id)}
															>
																<span class="block truncate">{source.name}</span>
															</button>
														</Sidebar.MenuItem>
													{/each}
												{:else}
													<div class="text-muted-foreground px-3 py-2 text-sm">无数据</div>
												{/if}
											{:else}
												<div class="text-muted-foreground px-3 py-2 text-sm">加载中...</div>
											{/if}
										</div>
									</Collapsible.Content>
								</Sidebar.MenuItem>
							</Collapsible.Root>
						{/each}
					</Sidebar.Menu>
				</Sidebar.GroupContent>
			</Sidebar.Group>
		</div>

		<!-- 固定在底部的设置选项 -->
		<div class="border-border mt-auto border-t pt-4">
			<Sidebar.Menu class="space-y-1">
				<Sidebar.MenuItem>
					<Sidebar.MenuButton asChild>
						<a
							href="/settings"
							class="hover:bg-accent/50 text-foreground flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 font-medium transition-all duration-200"
						>
							<div class="flex flex-1 items-center gap-3">
								<SettingsIcon class="text-muted-foreground h-4 w-4" />
								<span class="text-sm">设置</span>
							</div>
						</a>
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			</Sidebar.Menu>
		</div>
	</Sidebar.Content>
</Sidebar.Root>
