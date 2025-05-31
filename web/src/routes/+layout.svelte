<script lang="ts">
	import '../app.css';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
</script>

<Sidebar.Provider>
	<div class="flex min-h-screen w-full">
		<!-- 侧边栏 - 添加data属性 -->
		<div data-sidebar="sidebar">
			<AppSidebar />
		</div>

		<!-- 主内容区域 -->
		<Sidebar.Inset class="min-h-screen flex-1">
			<slot />
		</Sidebar.Inset>
	</div>
</Sidebar.Provider>
