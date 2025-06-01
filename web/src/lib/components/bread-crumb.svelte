<script lang="ts">
	import * as Breadcrumb from '$lib/components/ui/breadcrumb/index.js';

	export let items: Array<{
		href?: string;
		label: string;
		isActive?: boolean;
		onClick?: () => void;
	}> = [{ href: '/', label: '主页' }];
</script>

<Breadcrumb.Root>
	<Breadcrumb.List>
		{#each items as item, index (item.label)}
			<Breadcrumb.Item>
				{#if item.isActive || (!item.href && !item.onClick)}
					<Breadcrumb.Page>{item.label}</Breadcrumb.Page>
				{:else if item.onClick}
					<button
						class="hover:text-foreground cursor-pointer transition-colors"
						onclick={item.onClick}
					>
						{item.label}
					</button>
				{:else}
					<Breadcrumb.Link href={item.href}>{item.label}</Breadcrumb.Link>
				{/if}
			</Breadcrumb.Item>
			{#if index < items.length - 1}
				<Breadcrumb.Separator />
			{/if}
		{/each}
	</Breadcrumb.List>
</Breadcrumb.Root>
