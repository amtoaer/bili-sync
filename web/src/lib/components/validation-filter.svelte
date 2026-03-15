<script lang="ts">
	import {
		CircleCheckBigIcon,
		TriangleAlertIcon,
		SkipForwardIcon,
		ChevronDownIcon,
		TrashIcon
	} from '@lucide/svelte/icons';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { type ValidationFilterValue } from '$lib/stores/filter';

	interface Props {
		value: ValidationFilterValue;
		onSelect?: (value: ValidationFilterValue) => void;
		onRemove?: () => void;
	}

	let { value = $bindable('normal'), onSelect, onRemove }: Props = $props();

	let open = $state(false);
	let triggerRef = $state<HTMLButtonElement>(null!);

	function closeAndFocusTrigger() {
		open = false;
	}

	const validationOptions = [
		{
			value: 'normal' as const,
			label: '有效',
			icon: CircleCheckBigIcon
		},
		{
			value: 'skipped' as const,
			label: '跳过',
			icon: SkipForwardIcon
		},
		{
			value: 'invalid' as const,
			label: '失效',
			icon: TriangleAlertIcon
		}
	];

	function handleSelect(selectedValue: ValidationFilterValue) {
		value = selectedValue;
		onSelect?.(selectedValue);
		closeAndFocusTrigger();
	}

	const currentOption = $derived(validationOptions.find((opt) => opt.value === value));
</script>

<div class="inline-flex items-center gap-1">
	<span class="bg-secondary text-secondary-foreground rounded-lg px-2 py-1 text-xs font-medium">
		{currentOption ? currentOption.label : '未应用'}
	</span>

	<DropdownMenu.Root bind:open>
		<DropdownMenu.Trigger bind:ref={triggerRef}>
			{#snippet child({ props })}
				<Button variant="ghost" size="sm" {...props} class="h-6 w-6 p-0">
					<ChevronDownIcon class="h-3 w-3" />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-50" align="end">
			<DropdownMenu.Group>
				<DropdownMenu.Label class="text-xs">有效性</DropdownMenu.Label>
				{#each validationOptions as option (option.value)}
					<DropdownMenu.Item class="text-xs" onclick={() => handleSelect(option.value)}>
						<option.icon class="mr-2 size-3" />
						<span class:font-semibold={value === option.value}>
							{option.label}
						</span>
						{#if value === option.value}
							<CircleCheckBigIcon class="ml-auto size-3" />
						{/if}
					</DropdownMenu.Item>
				{/each}
				<DropdownMenu.Separator />
				<DropdownMenu.Item
					onclick={() => {
						closeAndFocusTrigger();
						onRemove?.();
					}}
				>
					<TrashIcon class="mr-2 size-3" />
					<span class="text-xs font-medium">移除筛选</span>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
