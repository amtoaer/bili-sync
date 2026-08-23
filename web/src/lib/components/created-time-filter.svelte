<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import EraserIcon from '@lucide/svelte/icons/eraser';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';

	interface Props {
		createdFrom: string | null;
		createdTo: string | null;
		onChange?: (createdFrom: string | null, createdTo: string | null) => void;
	}

	let { createdFrom, createdTo, onChange }: Props = $props();

	const id = $props.id();
	let open = $state(false);
	let draftCreatedFrom = $state('');
	let draftCreatedTo = $state('');

	const invalidRange = $derived(
		Boolean(draftCreatedFrom && draftCreatedTo && draftCreatedFrom > draftCreatedTo)
	);
	const displayValue = $derived.by(() => {
		const from = createdFrom?.slice(0, 10);
		const to = createdTo?.slice(0, 10);
		if (from && to) return `${from} – ${to}`;
		if (from) return `${from} 起`;
		if (to) return `截至 ${to}`;
		return '未应用';
	});

	$effect(() => {
		if (!open) {
			draftCreatedFrom = createdFrom || '';
			draftCreatedTo = createdTo || '';
		}
	});

	function apply() {
		if (invalidRange) return;
		onChange?.(
			draftCreatedFrom ? normalizeDateTime(draftCreatedFrom) : null,
			draftCreatedTo ? normalizeDateTime(draftCreatedTo) : null
		);
		open = false;
	}

	function normalizeDateTime(value: string) {
		return value.length === 16 ? `${value}:00` : value;
	}

	function clear() {
		draftCreatedFrom = '';
		draftCreatedTo = '';
	}
</script>

<div class="inline-flex items-center gap-1">
	<span
		class="bg-secondary text-secondary-foreground max-w-52 truncate rounded-lg px-2 py-1 text-xs font-medium"
		title={displayValue}
	>
		{displayValue}
	</span>

	<Popover.Root bind:open>
		<Popover.Trigger>
			{#snippet child({ props })}
				<Button variant="ghost" size="sm" {...props} class="h-6 w-6 p-0">
					<ChevronDownIcon class="h-3 w-3" />
				</Button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Content class="w-88" align="end">
			<div class="space-y-4">
				<div>
					<h4 class="text-sm leading-none font-medium">创建时间</h4>
					<p class="text-muted-foreground mt-1.5 text-xs">可只设置一侧，时间按服务器时区解释</p>
				</div>
				<div class="space-y-3">
					<div class="space-y-1.5">
						<Label for={`${id}-created-from`} class="text-xs">开始时间</Label>
						<Input
							id={`${id}-created-from`}
							type="datetime-local"
							class="text-xs"
							bind:value={draftCreatedFrom}
							max={draftCreatedTo || undefined}
						/>
					</div>
					<div class="space-y-1.5">
						<Label for={`${id}-created-to`} class="text-xs">结束时间</Label>
						<Input
							id={`${id}-created-to`}
							type="datetime-local"
							class="text-xs"
							bind:value={draftCreatedTo}
							min={draftCreatedFrom || undefined}
						/>
					</div>
					{#if invalidRange}
						<p class="text-destructive text-xs">结束时间不能早于开始时间</p>
					{/if}
				</div>
				<div class="flex items-center justify-between">
					<Button
						variant="ghost"
						size="sm"
						class="text-muted-foreground px-2 text-xs"
						disabled={!draftCreatedFrom && !draftCreatedTo}
						onclick={clear}
					>
						<EraserIcon class="size-3" />
						清空选择
					</Button>
					<div class="flex gap-2">
						<Button variant="outline" size="sm" class="text-xs" onclick={() => (open = false)}>
							取消
						</Button>
						<Button size="sm" class="text-xs" disabled={invalidRange} onclick={apply}>应用</Button>
					</div>
				</div>
			</div>
		</Popover.Content>
	</Popover.Root>
</div>
