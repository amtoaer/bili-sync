<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	let apiToken: string = localStorage.getItem('auth_token') || '';
	function updateToken() {
		localStorage.setItem('auth_token', apiToken);
	}
</script>

<header class="flex items-center justify-between bg-gray-100 p-4">
	<h1 class="text-xl font-bold">bili-sync 管理页</h1>
	<div>
		<Input placeholder="API Token" bind:value={apiToken} on:change={updateToken} />
	</div>
</header>
<slot />
