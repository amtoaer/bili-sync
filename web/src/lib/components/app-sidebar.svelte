<script lang="ts" module>
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import FileVideoIcon from '@lucide/svelte/icons/file-video';
	import BotIcon from '@lucide/svelte/icons/bot';
	import ChartPieIcon from '@lucide/svelte/icons/chart-pie';
	import LifeBuoyIcon from '@lucide/svelte/icons/life-buoy';
	import MapIcon from '@lucide/svelte/icons/map';
	import SendIcon from '@lucide/svelte/icons/send';
	import Settings2Icon from '@lucide/svelte/icons/settings-2';
	import SquareTerminalIcon from '@lucide/svelte/icons/square-terminal';

	const data = {
		header: {
			title: 'Bili Sync',
			subtitle: '后台管理系统',
			icon: BotIcon,
			href: '/'
		},
		navMain: [
			{
				category: '总览',
				items: [
					{
						title: '仪表盘',
						icon: ChartPieIcon,
						href: '/'
					},
					{
						title: '日志',
						icon: SquareTerminalIcon,
						href: '/logs'
					}
				]
			},
			{
				category: '内容管理',
				items: [
					{
						title: '视频',
						icon: FileVideoIcon,
						href: '/videos'
					},
					{
						title: '视频源',
						icon: DatabaseIcon,
						href: '/video-sources'
					}
				]
			},
			{
				category: '快捷订阅',
				items: [
					{
						title: '我创建的收藏夹',
						icon: Settings2Icon,
						href: '/me/favorites'
					},
					{
						title: '我关注的合集',
						icon: MapIcon,
						href: '/me/collections'
					},
					{
						title: '我关注的 up 主',
						icon: LifeBuoyIcon,
						href: '/me/uppers'
					}
				]
			}
		],
		footer: [
			{
				title: '设置',
				icon: Settings2Icon,
				href: '/settings'
			}
		]
	};
</script>

<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import CommandIcon from '@lucide/svelte/icons/command';
	import type { ComponentProps } from 'svelte';

	let { ref = $bindable(null), ...restProps }: ComponentProps<typeof Sidebar.Root> = $props();
</script>

<Sidebar.Root bind:ref variant="inset" {...restProps}>
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton size="lg">
					{#snippet child({ props })}
						<a href={data.header.href} {...props}>
							<div
								class="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg"
							>
								<data.header.icon class="size-4" />
							</div>
							<div class="grid flex-1 text-left text-sm leading-tight">
								<span class="truncate font-medium">{data.header.title}</span>
								<span class="truncate text-xs">{data.header.subtitle}</span>
							</div>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>
	<Sidebar.Content>
		<Sidebar.Group>
			{#each data.navMain as group (group.category)}
				<Sidebar.GroupLabel class="h-10">{group.category}</Sidebar.GroupLabel>
				<Sidebar.Menu>
					{#each group.items as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton class="h-8">
								{#snippet child({ props })}
									<a href={item.href} {...props}>
										<item.icon class="size-4" />
										<span class="text-sm">{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			{/each}
		</Sidebar.Group>
	</Sidebar.Content>
	<Sidebar.Footer>
		<Sidebar.Separator />
		<Sidebar.Menu>
			{#each data.footer as item (item.title)}
				<Sidebar.MenuItem>
					<Sidebar.MenuButton class="h-8">
						{#snippet child({ props })}
							<a href={item.href} {...props}>
								<item.icon class="size-4" />
								<span class="text-sm">{item.title}</span>
							</a>
						{/snippet}
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			{/each}
		</Sidebar.Menu>
	</Sidebar.Footer>
</Sidebar.Root>
