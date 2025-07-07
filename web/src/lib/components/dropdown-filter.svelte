<script lang="ts">
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import TrashIcon from '@lucide/svelte/icons/trash';
	import { tick } from 'svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Command from '$lib/components/ui/command/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	interface FilterValue {
		name: string;
		id: string;
	}

	interface Filter {
		key: string;
		name: string;
		icon: any;
		values: FilterValue[];
	}

	interface SelectedLabel {
		key: string;
		name: string;
		valueName: string;
		valueId: string;
	}

	interface Props {
		title: string;
		filters: Filter[];
		selectedLabel: SelectedLabel;
	}

	let { title = 'Actions', filters, selectedLabel = $bindable() }: Props = $props();

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
	<span class="bg-primary text-primary-foreground rounded-lg px-2 py-1 text-xs font-medium">
		{selectedLabel.valueName}
	</span>
	<DropdownMenu.Root bind:open>
		<DropdownMenu.Trigger bind:ref={triggerRef}>
			{#snippet child({ props })}
				<Button variant="ghost" size="sm" {...props} aria-label="Open menu" class="h-6 w-6 p-0">
					<EllipsisIcon class="h-3 w-3" />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-[200px]" align="end">
			<DropdownMenu.Group>
				{#each filters as filter (filter.key)}
					<DropdownMenu.Sub>
						<DropdownMenu.SubTrigger>
							{@render filter.icon({ class: 'mr-2 size-4' })}
							{filter.name}
						</DropdownMenu.SubTrigger>
						<DropdownMenu.SubContent class="p-0">
							<Command.Root value={selectedLabel.key === filter.key ? selectedLabel.valueId : ''}>
								<Command.Input autofocus placeholder="Filter {filter.name.toLowerCase()}..." />
								<Command.List>
									<Command.Empty>No {filter.name.toLowerCase()} found.</Command.Empty>
									<Command.Group>
										{#each filter.values as value (value.id)}
											<Command.Item
												value={value.id}
												onSelect={() => {
													selectedLabel = {
														key: filter.key,
														name: filter.name,
														valueName: value.name,
														valueId: value.id
													};
													closeAndFocusTrigger();
												}}
											>
												{value.name}
											</Command.Item>
										{/each}
									</Command.Group>
								</Command.List>
							</Command.Root>
						</DropdownMenu.SubContent>
					</DropdownMenu.Sub>
				{/each}
				<DropdownMenu.Separator />
				<DropdownMenu.Item class="text-red-600">
					<TrashIcon class="mr-2 size-4" />
					Delete
					<DropdownMenu.Shortcut>⌘⌫</DropdownMenu.Shortcut>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
