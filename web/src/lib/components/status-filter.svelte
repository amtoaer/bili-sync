<script lang="ts">
	import {
		CircleCheckBigIcon,
		CircleXIcon,
		ClockIcon,
		ChevronDownIcon,
		TrashIcon
	} from '@lucide/svelte/icons';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { type StatusFilterValue } from '$lib/stores/filter';

	interface Props {
		value: StatusFilterValue | null;
		onSelect?: (value: StatusFilterValue) => void;
		onRemove?: () => void;
	}

	let { value = $bindable(null), onSelect, onRemove }: Props = $props();

	let open = $state(false);
	let triggerRef = $state<HTMLButtonElement>(null!);

	function closeAndFocusTrigger() {
		open = false;
	}

	const statusOptions = [
		{
			value: 'failed' as const,
			label: '仅失败',
			icon: CircleXIcon
		},
		{
			value: 'succeeded' as const,
			label: '仅成功',
			icon: CircleCheckBigIcon
		},
		{
			value: 'waiting' as const,
			label: '仅等待',
			icon: ClockIcon
		}
	];

	function handleSelect(selectedValue: StatusFilterValue) {
		value = selectedValue;
		onSelect?.(selectedValue);
		closeAndFocusTrigger();
	}

	const currentOption = $derived(statusOptions.find((opt) => opt.value === value));
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
				<DropdownMenu.Label class="text-xs">视频状态</DropdownMenu.Label>
				{#each statusOptions as option (option.value)}
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
