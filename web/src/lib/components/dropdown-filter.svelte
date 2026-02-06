<script lang="ts">
	import { EllipsisIcon, TrashIcon } from '@lucide/svelte/icons';
	import { tick } from 'svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Command from '$lib/components/ui/command/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	export interface Filter {
		name: string;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		icon: any;
		values: Record<string, string>;
	}

	interface SelectedLabel {
		type: string;
		id: string;
	}

	interface Props {
		filters: Record<string, Filter> | null;
		selectedLabel: SelectedLabel | null;
		onSelect?: (type: string, id: string) => void;
		onRemove?: () => void;
	}

	let { filters, selectedLabel = $bindable(), onSelect, onRemove }: Props = $props();

	let open = $state(false);
	let triggerRef = $state<HTMLButtonElement>(null!);

	// We want to refocus the trigger button when the user selects
	// an item from the list so users can continue navigating the
	// rest of the form with the keyboard.
	function closeAndFocusTrigger() {
		open = false;
		tick().then(() => {
			triggerRef.focus();
		});
	}
</script>

<div class="inline-flex items-center gap-1">
	{#if filters}
		<span class="bg-secondary text-secondary-foreground rounded-lg px-2 py-1 text-xs font-medium">
			{#if selectedLabel && selectedLabel.type && selectedLabel.id}
				「{filters[selectedLabel.type]?.name || ''} : {filters[selectedLabel.type]!.values[
					selectedLabel.id
				] || ''}」
			{:else}
				未应用
			{/if}
		</span>
	{/if}

	<DropdownMenu.Root bind:open>
		<DropdownMenu.Trigger bind:ref={triggerRef}>
			{#snippet child({ props })}
				<Button variant="ghost" size="sm" {...props} class="h-6 w-6 p-0">
					<EllipsisIcon class="h-3 w-3" />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-50" align="end">
			<DropdownMenu.Group>
				{#if filters}
					{#each Object.entries(filters) as [key, filter] (key)}
						<DropdownMenu.Sub>
							<DropdownMenu.SubTrigger>
								<filter.icon class="mr-2 size-3" />
								<span class="text-xs font-medium">
									{filter.name}
								</span>
							</DropdownMenu.SubTrigger>
							<DropdownMenu.SubContent class="p-0">
								<Command.Root
									value={selectedLabel && selectedLabel.type === key ? selectedLabel.id : ''}
								>
									<Command.Input
										class="text-xs"
										autofocus
										placeholder="查找{filter.name.toLowerCase()}..."
									/>
									<Command.List>
										<Command.Empty class="text-xs"
											>未找到"{filter.name.toLowerCase()}"</Command.Empty
										>
										<Command.Group>
											{#each Object.entries(filter.values) as [id, name] (id)}
												<Command.Item
													value={name}
													class="text-xs"
													onSelect={() => {
														closeAndFocusTrigger();
														onSelect?.(key, id);
													}}
												>
													{name}
												</Command.Item>
											{/each}
										</Command.Group>
									</Command.List>
								</Command.Root>
							</DropdownMenu.SubContent>
						</DropdownMenu.Sub>
					{/each}
				{/if}
				<DropdownMenu.Separator />
				<DropdownMenu.Item
					onclick={() => {
						closeAndFocusTrigger();
						onRemove?.();
					}}
				>
					<TrashIcon class="mr-2 size-3" />
					<span class="text-xs font-medium"> 移除筛选 </span>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
