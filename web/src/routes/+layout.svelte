<script lang="ts">
	import '../app.css';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { breadcrumbStore } from '$lib/stores/breadcrumb';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { Toaster } from '$lib/components/ui/sonner/index.js';
	import { ModeWatcher } from 'mode-watcher';
</script>

<ModeWatcher disableHeadScriptInjection />
<Toaster />
<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset class="flex flex-col" style="height: calc(100vh - 1rem)">
		<header class="flex h-16 shrink-0 items-center gap-2">
			<div class="flex items-center gap-2 px-4">
				<Sidebar.Trigger class="-ml-1" />
				<Separator orientation="vertical" class="mr-2 data-[orientation=vertical]:h-4" />
				<BreadCrumb items={$breadcrumbStore} />
			</div>
		</header>
		<div
			class="w-full overflow-y-auto px-6 py-2"
			style="scrollbar-width: thin; scrollbar-gutter: stable !important;"
			id="main"
		>
			<slot />
		</div>
	</Sidebar.Inset>
</Sidebar.Provider>
