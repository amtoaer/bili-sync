<script lang="ts">
	import '../app.css';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
</script>

<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset>
		<header class="flex h-16 shrink-0 items-center gap-2">
			<div class="flex items-center gap-2 px-4">
				<Sidebar.Trigger class="-ml-1" />
				<Separator orientation="vertical" class="mr-2 data-[orientation=vertical]:h-4" />
				<BreadCrumb />
			</div>
		</header>
		<div class="bg-background min-h-screen w-full">
			<div class="w-full px-6 py-6">
				<slot />
			</div>
		</div>
	</Sidebar.Inset>
</Sidebar.Provider>
