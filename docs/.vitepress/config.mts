import { defineConfig } from "vitepress";
import taskLists from "markdown-it-task-lists";

// https://vitepress.dev/reference/site-config
export default defineConfig({
	title: "bili-sync",
	description: "基于 rust tokio 编写的 bilibili 收藏夹同步下载工具",
	lang: "zh-CN",
	themeConfig: {
		outline: {
			label: "目录跳转",
			level: "deep",
		},
		// https://vitepress.dev/reference/default-theme-config
		nav: [
			{ text: "主页", link: "/" },
			{ text: "示例", link: "/markdown-examples" },
		],

		sidebar: [
			{
				text: "简介",
				items: [
					{ text: "什么是 bili-sync？", link: "/contents/intro" },
					{ text: "快速开始", link: "/contents/quick-start" },
				],
			},
		],

		socialLinks: [
			{ icon: "github", link: "https://github.com/amtoaer/bili-sync" },
		],
	},
	markdown: {
		config: (md) => {
			md.use(taskLists);
		},
	},
});
