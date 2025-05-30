<script lang="ts">
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import HouseIcon from '@lucide/svelte/icons/house';
	import InboxIcon from '@lucide/svelte/icons/inbox';
	import SearchIcon from '@lucide/svelte/icons/search';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import SidebarHeader from './ui/sidebar/sidebar-header.svelte';

	import { onMount } from 'svelte';
	import type { VideoSourcesResponse } from '$lib/types';
	import api from '$lib/api';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';

	let video_sources: VideoSourcesResponse | null = null;

	onMount(async () => {
		video_sources = (await api.getVideoSources()).data;
	});

	const items = [
		{
			title: '收藏夹',
			type: 'favorite',
			url: '#',
			icon: HouseIcon
		},
		{
			title: '合集/列表',
			type: 'collection',
			url: '#',
			icon: InboxIcon
		},
		{
			title: '用户投稿',
			type: 'submission',
			url: '#',
			icon: CalendarIcon
		},
		{
			title: '稍后再看',
			type: 'watch_later',
			url: '#',
			icon: SearchIcon
		}
	];
</script>

<Sidebar.Root>
	<SidebarHeader>
		<div class="flex items-center gap-2 px-4 py-2">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				class="h-5 w-5"
			>
				<path d="m22 8-6 4 6 4V8Z" />
				<rect width="14" height="12" x="2" y="6" rx="2" ry="2" />
			</svg>
			<span class="text-sm font-semibold">bili-sync 管理面板</span>
		</div>
	</SidebarHeader>
	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupLabel>视频来源</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each items as item (item.type)}
						<Collapsible.Root open class="group/collapsible">
							<Sidebar.MenuItem>
								<Collapsible.Trigger>
									{#snippet child({ props })}
										<Sidebar.MenuButton {...props}
											>{#snippet child({ props })}
												<a href={item.url} {...props}>
													<item.icon />
													<span>{item.title}</span>
												</a>
											{/snippet}</Sidebar.MenuButton
										>{/snippet}
								</Collapsible.Trigger>
								<Collapsible.Content>
									{#if video_sources}
										{#each video_sources[item.type as keyof VideoSourcesResponse] as source (source.id)}
											<Sidebar.MenuItem>
												<Sidebar.MenuButton>
													{#snippet child({ props })}
														<a href="/#" {...props}>
															<span>{source.name}</span>
														</a>
													{/snippet}
												</Sidebar.MenuButton>
											</Sidebar.MenuItem>
										{/each}
									{:else}
										<Sidebar.MenuItem>
											<div
												class="items flex
												justify-center px-4 py-2 text-sm text-gray-500"
											>
												加载中...
											</div>
										</Sidebar.MenuItem>
									{/if}
								</Collapsible.Content>
							</Sidebar.MenuItem></Collapsible.Root
						>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>
</Sidebar.Root>
