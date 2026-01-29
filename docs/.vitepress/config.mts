import { defineConfig } from "vitepress";
import taskLists from "markdown-it-task-lists";

// https://vitepress.dev/reference/site-config
export default defineConfig({
	title: "bili-sync",
	description: "由 Rust & Tokio 驱动的哔哩哔哩同步工具",
	lang: "zh-Hans",
	sitemap: {
		hostname: "https://bili-sync.github.io",
	},
	lastUpdated: true,
	cleanUrls: true,
	metaChunk: true,
	themeConfig: {
		outline: {
			label: "页面导航",
			level: "deep",
		},
		// https://vitepress.dev/reference/default-theme-config
		nav: [
			{ text: "主页", link: "/" },
			{
				text: "v2.10.3",
				items: [
					{
						text: "程序更新",
						link: "https://github.com/amtoaer/bili-sync/releases",
					},
					{
						text: "文档更新",
						link: "https://github.com/search?q=repo:amtoaer/bili-sync+docs&type=commits",
					},
				],
			},
		],
		sidebar: [
			{
				text: "简介",
				items: [
					{ text: "什么是 bili-sync？", link: "/introduction" },
					{ text: "快速开始", link: "/quick-start" },
				],
			},
			{
				text: "细节",
				items: [
					{ text: "配置说明", link: "/configuration" },
					{ text: "命令行参数", link: "/args" },
					{ text: "工作原理", link: "/design" },
				],
			},
			{
				text: "参考",
				items: [
					{ text: "获取收藏夹信息", link: "/favorite" },
					{
						text: "获取合集/列表信息",
						link: "/collection",
					},
					{ text: "获取用户投稿信息", link: "/submission" },
				],
			},
			{
				text: "其它",
				items: [
					{ text: "常见问题", link: "/question" },
					{ text: "管理页", link: "/frontend" },
				],
			}
		],
		socialLinks: [
			{ icon: "github", link: "https://github.com/amtoaer/bili-sync" },
		],
		search: {
			provider: "local",
		},
		notFound: {
			title: "你来到了没有知识的荒原",
			quote: "这里什么都没有",
			linkText: "返回首页",
		},
		docFooter: {
			prev: "上一页",
			next: "下一页",
		},
		lastUpdated: {
			text: "上次更新于",
		},
		returnToTopLabel: "回到顶部",
		sidebarMenuLabel: "菜单",
		darkModeSwitchLabel: "主题",
		lightModeSwitchTitle: "切换到浅色模式",
		darkModeSwitchTitle: "切换到深色模式",
	},
	markdown: {
		config: (md) => {
			md.use(taskLists);
		},
		theme: {
			light: "github-light",
			dark: "github-dark",
		},
	},
	head: [
		["link", { rel: "icon", type: "image/svg+xml", href: "/icon.svg" }],
		["link", { rel: "icon", type: "image/png", href: "/icon.png" }],
	],
});
