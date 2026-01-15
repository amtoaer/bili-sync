<script lang="ts">
	import CheckCircleIcon from '@lucide/svelte/icons/check-circle';
	import XCircleIcon from '@lucide/svelte/icons/x-circle';
	import CircleIcon from '@lucide/svelte/icons/circle';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { type StatusFilterValue } from '$lib/stores/filter';

	interface Props {
		value: StatusFilterValue;
		onSelect?: (value: StatusFilterValue) => void;
	}

	let { value = $bindable('all'), onSelect }: Props = $props();

	let open = $state(false);
	let triggerRef = $state<HTMLButtonElement>(null!);

	function closeAndFocusTrigger() {
		open = false;
	}

	const statusOptions = [
		{
			value: 'all' as const,
			label: '全部',
			icon: CircleIcon
		},
		{
			value: 'failed' as const,
			label: '仅失败',
			icon: XCircleIcon
		},
		{
			value: 'succeeded' as const,
			label: '仅成功',
			icon: CheckCircleIcon
		},
		{
			value: 'waiting' as const,
			label: '等待中',
			icon: ClockIcon
		}
	];

	function handleSelect(selectedValue: StatusFilterValue) {
		value = selectedValue;
		onSelect?.(selectedValue);
		closeAndFocusTrigger();
	}

	const currentOption = $derived(
		statusOptions.find((opt) => opt.value === value) || statusOptions[0]
	);
</script>

<div class="inline-flex items-center gap-1">
	<span class="bg-secondary text-secondary-foreground rounded-lg px-2 py-1 text-xs font-medium">
		{currentOption.label}
	</span>

	<DropdownMenu.Root bind:open>
		<DropdownMenu.Trigger bind:ref={triggerRef}>
			{#snippet child({ props })}
				<Button variant="ghost" size="sm" {...props} class="h-6 w-6 p-0">
					<ChevronDownIcon class="h-3 w-3" />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-[140px]" align="end">
			<DropdownMenu.Group>
				<DropdownMenu.Label class="text-xs">视频状态</DropdownMenu.Label>
				{#each statusOptions as option (option.value)}
					<DropdownMenu.Item class="text-xs" onclick={() => handleSelect(option.value)}>
						<option.icon class="mr-2 size-3" />
						<span class:font-semibold={value === option.value}>
							{option.label}
						</span>
						{#if value === option.value}
							<CheckCircleIcon class="ml-auto size-3" />
						{/if}
					</DropdownMenu.Item>
				{/each}
			</DropdownMenu.Group>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
